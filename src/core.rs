use std::io::{self, BufRead, BufWriter, IsTerminal, LineWriter, Write};
use std::process::ExitCode;

use crate::files;
use crate::output::{Display, OutputStyle};
use crate::params;
use crate::scanner;

/// Entry point for the application, called from main().
pub fn run(params: &params::Parameters) -> io::Result<ExitCode> {
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
    if params.show_context.after != 0 {
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
    let with_color = isatty; // or colorchoice::ColorChoice::global() == ???

    // Line-buffered or not.
    let mut writer: Box<dyn Write> = if params.line_buffered || isatty {
        Box::new(LineWriter::new(stdout.lock()))
    } else {
        Box::new(BufWriter::new(stdout.lock()))
    };

    // Create scanner that knows what to expect.
    let netcandidatescanner = scanner::NetCandidateScanner::new()
        .ignore_ipv4(params.needles.iter().all(|n| !n.net.is_ipv4()))
        .ignore_ipv6(params.needles.iter().all(|n| !n.net.is_ipv6()))
        .set_accept(params.accept)
        .set_interface_mode(params.interface_mode);

    // Create display that knows how to output.
    let disp = Display::new()
        .show_filename(show_filename)
        .show_lineno(params.show_lineno)
        .show_color(with_color);

    let mut any_match = false;

    for file_res in file_iter {
        let mut file = match file_res {
            Ok(o) => o,
            Err(e) => {
                eprintln!("ipgrep: {e}");
                continue;
            }
        };

        let match_count = search_in_file(
            &disp,
            &mut file,
            &netcandidatescanner,
            params,
            &mut writer,
        )?;

        any_match = any_match || (match_count != 0);

        match params.output_style {
            OutputStyle::JustExitCode => {
                if match_count != 0 {
                    break;
                }
            }
            OutputStyle::ShowFilesWithLf => {
                if match_count != 0 {
                    disp.print_filename(&mut writer, &file.name, b"\n")?;
                }
            }
            OutputStyle::ShowFilesWithNull => {
                if match_count != 0 {
                    disp.print_filename(&mut writer, &file.name, b"\0")?;
                }
            }
            OutputStyle::ShowCountsPerFile => {
                disp.print_counts(&mut writer, &file.name, match_count)?;
            }
            OutputStyle::ShowOnlyMatching => {}
            OutputStyle::ShowLinesAndContext => {}
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

fn search_in_file(
    disp: &Display,
    file: &mut files::FileSource,
    netcandidatescanner: &scanner::NetCandidateScanner,
    params: &params::Parameters,
    writer: &mut dyn Write,
) -> io::Result<usize> {
    let mut line = Vec::new();
    let mut lineno = 0;

    let mut matches = Vec::new();
    let mut match_count: usize = 0;

    loop {
        // TODO: This could use some test case. But it looks like it
        // works, even including files without trailing newlines.
        let _n = match file.reader.read_until(b'\n', &mut line) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                eprintln!("ipgrep: {}: {} (skipping)", file.name, e);
                break;
            }
        };
        lineno += 1;

        for candidate in netcandidatescanner.find_all(&line) {
            for needle in &params.needles {
                if params.match_mode.matches(&candidate.net, &needle.net) {
                    matches.push(candidate);
                    // Push once per needle only. Makes no sense to
                    // have the same match twice.
                    break;
                }
            }
        }

        if !matches.is_empty() {
            match_count += matches.len();

            match params.output_style {
                OutputStyle::JustExitCode
                | OutputStyle::ShowFilesWithLf
                | OutputStyle::ShowFilesWithNull => {
                    // Short circuit. Don't trust the match_count, so
                    // set it to 1.
                    match_count = 1;
                    break;
                }
                OutputStyle::ShowCountsPerFile => {}
                OutputStyle::ShowOnlyMatching => {
                    disp.print_matches(
                        writer, &file.name, lineno, &line, &matches,
                    )?;
                }
                OutputStyle::ShowLinesAndContext => {
                    // FIXME: add the missing context
                    disp.print_line(
                        writer, &file.name, lineno, &line, &matches,
                    )?;
                }
            }

            matches.clear();
        }
        line.clear();
    }

    Ok(match_count)
}
