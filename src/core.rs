use std::io::{self, BufRead, BufWriter, IsTerminal, LineWriter, Write};
use std::process::ExitCode;

use crate::files;
use crate::output::{Display, OutputStyle};
use crate::params;
use crate::scanner;

/// Entry point for the application, called from main().
pub fn run(params: &params::Parameters) -> io::Result<ExitCode> {
    //eprintln!("DBG: {:?}", params);

    let needles = &params.needles;
    //eprintln!("DBG: {:?}", needles);

    let file_iter = if params.haystack_filenames.is_empty() {
        files::FileSourceIter::new().add_stdin()
    } else {
        files::FileSourceIter::new()
            .set_recursion(params.recursive)
            .add_files(&params.haystack_filenames)
    };

    // FIXME: check for incomplete functionality
    if params.accept.oldnet {
        panic!("AcceptSet.oldnet is not really supported yet");
    }
    if params.show_context.before != 0 {
        panic!("Context.before is not really supported yet");
    }
    if params.show_context.before != 0 {
        panic!("Context.after is not really supported yet");
    }

    // GNU grep 3 compatibility:
    // - by default, no filename is shown;
    // - for more than one file (including recursion), we show;
    // - unless it is explicitly hidden.
    let show_filename = if params.hide_filename {
        false
    } else {
        file_iter.has_more_than_one_file()
    };

    let stdout = io::stdout();
    let isatty = stdout.is_terminal();
    let with_color = isatty;

    // Line-buffered?
    let mut writer: Box<dyn Write> = if params.line_buffered || isatty {
        Box::new(LineWriter::new(stdout.lock()))
    } else {
        Box::new(BufWriter::new(stdout.lock()))
    };

    // Create scanner that knows what to expect.
    let netcandidatescanner = {
        let include_ipv4 = params.needles.iter().any(|n| n.net.is_ipv4());
        let include_ipv6 = params.needles.iter().any(|n| n.net.is_ipv6());
        scanner::NetCandidateScanner::new(
            include_ipv4,
            include_ipv6,
            params.accept,
            params.interface_mode,
        )
    };

    // Create display that knows how to output.
    let disp = Display::new()
        .show_filename(show_filename)
        .show_lineno(params.show_lineno)
        .show_color(with_color);

    let mut any_match = false;

    for file_res in file_iter {
        let file = match file_res {
            Ok(o) => o,
            Err(e) => {
                eprintln!("ERR: {e}");
                continue;
            }
        };

        let mut reader = file.reader;
        let mut line = Vec::new();
        let mut lineno = 0;

        let mut matches = Vec::new();
        let mut match_count = 0;

        loop {
            line.clear();

            // TODO: This could use some test case. But it looks like it
            // works, even including files without trailing newlines.
            let _n = match reader.read_until(b'\n', &mut line) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    eprintln!("ERR: {}: {} (skipping)", file.name, e);
                    break;
                }
            };
            lineno += 1;

            for candidate in netcandidatescanner.find_all(&line) {
                for needle in needles {
                    if params.match_mode.matches(&candidate.net, &needle.net) {
                        matches.push(candidate);
                        break;
                    }
                }
            }

            if !matches.is_empty() {
                any_match = true;

                match params.output_style {
                    OutputStyle::ShowNothing => break,
                    OutputStyle::ShowFilesOnlyNull => {
                        disp.print_filename(&mut writer, &file.name, b"\0")?;
                        break;
                    }
                    OutputStyle::ShowFilesOnly => {
                        disp.print_filename(&mut writer, &file.name, b"\n")?;
                        break;
                    }
                    OutputStyle::ShowCountsPerFile => {
                        match_count += matches.len();
                    }
                    OutputStyle::ShowOnlyMatching => {
                        disp.print_matches(
                            &mut writer,
                            &file.name,
                            lineno,
                            &line,
                            &matches,
                        )?;
                    }
                    OutputStyle::ShowLinesAndContext => {
                        // FIXME: add the missing context
                        disp.print_line(
                            &mut writer,
                            &file.name,
                            lineno,
                            &line,
                            &matches,
                        )?;
                    }
                }

                matches.clear();
            }
        }

        match params.output_style {
            OutputStyle::ShowNothing => {
                if any_match {
                    break;
                }
            }
            OutputStyle::ShowFilesOnlyNull | OutputStyle::ShowFilesOnly => {}
            OutputStyle::ShowCountsPerFile => {
                disp.print_counts(&mut writer, &file.name, match_count)?;
            }
            OutputStyle::ShowOnlyMatching
            | OutputStyle::ShowLinesAndContext => {}
        }
    }

    let exit = match any_match {
        true => ExitCode::SUCCESS,
        false => ExitCode::from(1),
    };

    // Flush, just in case.
    writer.flush().ok();

    Ok(exit)
}
