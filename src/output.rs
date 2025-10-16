use std::io::{self, Write};

use crate::scanner::NetCandidate;

// From GNU grep 3 manual:
//
// GREP_COLORS=ms=01;31:mc=01;31:sl=:cx=:fn=35:ln=32:bn=32:se=36
// ...
// mt=01;31 ("match", "bold red")
// fn=35 ("filename", "magenta")
// ln=32 ("lineno", "green"), or bn=32 ("byte-offset", "green")
// se=36 ("separator", "cyan")
//
const COLOR_MATCH: &str = "\x1b[1;31m"; // red <results>
const COLOR_FILENAME: &str = "\x1b[0;35m"; // purple <filename>
const COLOR_LINENO: &str = "\x1b[0;32m"; // green <number>
const COLOR_SEPARATOR: &str = "\x1b[0;36m"; // cyan ":"/"-"
const COLOR_RESET: &str = "\x1b[0m";

/// Output modes, in order of precedence
#[derive(Debug, PartialEq)]
pub enum OutputStyle {
    // Only exit with status
    JustExitCode,
    // Only files that have a match
    ShowFilesWithLf,
    // Only files that have a match, NUL terminated
    ShowFilesWithNull,
    // All files, and a count of matches
    ShowCountsPerFile,
    // Show only the matches (no lines, no context)
    ShowOnlyMatching,
    // Show the lines (and optional context)
    ShowLinesAndContext,
}

pub struct Display {
    show_filename: bool,
    show_lineno: bool,
    show_color: bool,
}

impl Display {
    pub fn new() -> Self {
        Self {
            show_color: false,
            show_filename: false,
            show_lineno: false,
        }
    }

    pub fn show_color(self, value: bool) -> Self {
        Self {
            show_color: value,
            ..self
        }
    }

    pub fn show_filename(self, value: bool) -> Self {
        Self {
            show_filename: value,
            ..self
        }
    }

    pub fn show_lineno(self, value: bool) -> Self {
        Self {
            show_lineno: value,
            ..self
        }
    }

    pub fn print_filename(
        &self,
        writer: &mut dyn Write,
        filename: &str,
        end: &[u8],
    ) -> io::Result<()> {
        self.write_filename(writer, filename)?;
        self.write_no_color(writer)?;
        self.write(writer, end)?;
        Ok(())
    }

    pub fn print_counts(
        &self,
        writer: &mut dyn Write,
        filename: &str,
        count: usize,
    ) -> io::Result<()> {
        if self.show_filename {
            self.write_filename(writer, filename)?;
            self.write_separator(writer, b":")?;
            self.write_no_color(writer)?;
        }
        self.write_count(writer, count)?;
        Ok(())
    }

    pub fn print_matches(
        &self,
        writer: &mut dyn Write,
        filename: &str,
        lineno: usize,
        line: &[u8],
        matches: &Vec<NetCandidate>,
    ) -> io::Result<()> {
        for match_ in matches {
            if self.show_filename {
                self.write_filename(writer, filename)?;
                self.write_separator(writer, b":")?;
            }
            if self.show_lineno {
                self.write_linenumber(writer, lineno)?;
                self.write_separator(writer, b":")?;
            }
            self.write_match(writer, line, match_)?;
            self.write_no_color(writer)?;
            self.write(writer, b"\n")?;
        }
        Ok(())
    }

    pub fn print_context(
        &self,
        writer: &mut dyn Write,
        filename: &str,
        lineno: usize,
        line: &[u8],
    ) -> io::Result<()> {
        if self.show_filename {
            self.write_filename(writer, filename)?;
            self.write_separator(writer, b"-")?;
        }
        if self.show_lineno {
            self.write_linenumber(writer, lineno)?;
            self.write_separator(writer, b"-")?;
        }
        if self.show_filename || self.show_lineno {
            self.write_no_color(writer)?;
        }
        self.write(writer, line)?;
        Ok(())
    }

    pub fn print_context_delimiter(
        &self,
        writer: &mut dyn Write,
        _filename: &str,
        _lineno: usize,
    ) -> io::Result<()> {
        self.write_separator(writer, b"--\n")?; // delimiter "--"
        self.write_no_color(writer)?;
        Ok(())
    }

    pub fn print_line(
        &self,
        writer: &mut dyn Write,
        filename: &str,
        lineno: usize,
        line: &[u8],
        matches: &Vec<NetCandidate>,
    ) -> io::Result<()> {
        if self.show_filename {
            self.write_filename(writer, filename)?;
            self.write_separator(writer, b":")?;
        }
        if self.show_lineno {
            self.write_linenumber(writer, lineno)?;
            self.write_separator(writer, b":")?;
        }
        if self.show_filename || self.show_lineno {
            self.write_no_color(writer)?;
        }
        self.write_line(writer, line, matches)?;
        Ok(())
    }

    #[inline]
    fn write(&self, writer: &mut dyn Write, value: &[u8]) -> io::Result<()> {
        writer.write_all(value)?;
        Ok(())
    }

    #[inline]
    fn write_no_color(&self, writer: &mut dyn Write) -> io::Result<()> {
        if self.show_color {
            writer.write_all(COLOR_RESET.as_bytes())?;
        }
        Ok(())
    }

    #[inline]
    fn write_count(
        &self,
        writer: &mut dyn Write,
        count: usize,
    ) -> io::Result<()> {
        writer.write_all(format!("{count}\n").as_bytes())?;
        Ok(())
    }

    #[inline]
    fn write_separator(
        &self,
        writer: &mut dyn Write,
        delim: &[u8],
    ) -> io::Result<()> {
        if self.show_color {
            writer.write_all(COLOR_SEPARATOR.as_bytes())?;
        }
        writer.write_all(delim)?;
        Ok(())
    }

    #[inline]
    fn write_filename(
        &self,
        writer: &mut dyn Write,
        filename: &str,
    ) -> io::Result<()> {
        if self.show_color {
            writer.write_all(COLOR_FILENAME.as_bytes())?;
        }
        writer.write_all(filename.as_bytes())?;
        Ok(())
    }

    #[inline]
    fn write_match(
        &self,
        writer: &mut dyn Write,
        line: &[u8],
        match_: &NetCandidate,
    ) -> io::Result<()> {
        if self.show_color {
            writer.write_all(COLOR_MATCH.as_bytes())?;
        }
        let start = match_.range.0;
        let end = match_.range.1;
        writer.write_all(&line[start..end.min(line.len())])?;
        Ok(())
    }

    #[inline]
    fn write_linenumber(
        &self,
        writer: &mut dyn Write,
        lineno: usize,
    ) -> io::Result<()> {
        if self.show_color {
            writer.write_all(COLOR_LINENO.as_bytes())?;
        }
        writer.write_all(format!("{lineno}").as_bytes())?;
        Ok(())
    }

    #[inline]
    fn write_line(
        &self,
        writer: &mut dyn Write,
        line: &[u8],
        matches: &Vec<NetCandidate>,
    ) -> io::Result<()> {
        if self.show_color {
            let mut cursor = 0;
            for match_ in matches {
                let start = match_.range.0;
                let end = match_.range.1;

                // write text before the match
                if cursor < start {
                    writer.write_all(&line[cursor..start])?;
                }

                // write the colored match itself
                writer.write_all(COLOR_MATCH.as_bytes())?;
                writer.write_all(&line[start..end.min(line.len())])?;
                writer.write_all(COLOR_RESET.as_bytes())?;

                cursor = end;
            }

            // write the rest (after last match)
            if cursor < line.len() {
                writer.write_all(&line[cursor..])?;
            }
        } else {
            writer.write_all(line)?;
        }
        Ok(())
    }
}

impl Default for Display {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    use crate::net::Net;

    /// Helper that runs a test for both color modes and compares output.
    fn check_display<F>(mut disp: Display, expected: &str, mut do_display: F)
    where
        F: FnMut(&mut Display, &mut Vec<u8>) -> io::Result<()>,
    {
        let re = Regex::new(r"\x1b\[[0-9;]*m").unwrap();

        for show_color in [false, true] {
            disp = disp.show_color(show_color);

            let mut output = Vec::new();
            do_display(&mut disp, &mut output).expect("write failed");

            let got = String::from_utf8(output).expect("utf8");

            if show_color {
                assert_eq!(got, expected, "mismatch (with color)");
            } else {
                let no_color = re.replace_all(expected, "").into_owned();
                assert_eq!(got, no_color, "mismatch (no color)");
            }
        }
    }

    #[test]
    fn display_print_filename() {
        check_display(
            Display::new(),
            "\u{1b}[0;35m./path/to/example.txt\u{1b}[0m\n",
            |d, o| d.print_filename(o, "./path/to/example.txt", b"\n"),
        );
    }

    #[test]
    fn display_print_filename_null() {
        check_display(
            Display::new(),
            "\u{1b}[0;35mpath_with_NUL_at_EOL\u{1b}[0m\0",
            |d, o| d.print_filename(o, "path_with_NUL_at_EOL", b"\0"),
        );
    }

    #[test]
    fn display_print_counts_no_filename() {
        check_display(Display::new(), "42\n", |d, o| {
            d.print_counts(o, "filename not shown", 42)
        });
    }

    #[test]
    fn display_print_counts_with_filename() {
        check_display(
            Display::new().show_filename(true),
            "\u{1b}[0;35mfilename is shown\u{1b}[0;36m:\u{1b}[0m43\n",
            |d, o| d.print_counts(o, "filename is shown", 43),
        );
    }

    #[test]
    fn display_print_matches() {
        let line = b"nets: 10.20.30.1-10.20.30.20 <--\n";
        let matches = vec![
            NetCandidate {
                range: (6, 16),
                net: Net::from_str_unchecked("10.20.30.1"),
            },
            NetCandidate {
                range: (17, 28),
                net: Net::from_str_unchecked("10.20.30.20"),
            },
        ];
        check_display(
            Display::new(),
            "\u{1b}[1;31m10.20.30.1\u{1b}[0m\n\
             \u{1b}[1;31m10.20.30.20\u{1b}[0m\n",
            |d, o| d.print_matches(o, "fn", 351, line, &matches),
        );
        check_display(
            Display::new().show_filename(true),
            "\u{1b}[0;35mfn\u{1b}[0;36m:\u{1b}[1;31m10.20.30.1\u{1b}[0m\n\
             \u{1b}[0;35mfn\u{1b}[0;36m:\u{1b}[1;31m10.20.30.20\u{1b}[0m\n",
            |d, o| d.print_matches(o, "fn", 352, line, &matches),
        );
        check_display(
            Display::new().show_lineno(true),
            "\u{1b}[0;32m353\u{1b}[0;36m:\u{1b}[1;31m10.20.30.1\u{1b}[0m\n\
             \u{1b}[0;32m353\u{1b}[0;36m:\u{1b}[1;31m10.20.30.20\u{1b}[0m\n",
            |d, o| d.print_matches(o, "fn", 353, line, &matches),
        );
        check_display(
            Display::new().show_filename(true).show_lineno(true),
            "\u{1b}[0;35mfn\u{1b}[0;36m:\u{1b}[0;32m354\u{1b}[0;36m\
             :\u{1b}[1;31m10.20.30.1\u{1b}[0m\n\
             \u{1b}[0;35mfn\u{1b}[0;36m:\u{1b}[0;32m354\u{1b}[0;36m\
             :\u{1b}[1;31m10.20.30.20\u{1b}[0m\n",
            |d, o| d.print_matches(o, "fn", 354, line, &matches),
        );
    }

    #[test]
    fn display_print_context() {
        let line = b"whatever context\n";
        check_display(Display::new(), "whatever context\n", |d, o| {
            d.print_context(o, "fn", 1231, line)
        });
        check_display(
            Display::new().show_filename(true),
            "\u{1b}[0;35mfnX\u{1b}[0;36m-\u{1b}[0mwhatever context\n",
            |d, o| d.print_context(o, "fnX", 1232, line),
        );
        check_display(
            Display::new().show_lineno(true),
            "\u{1b}[0;32m1233\u{1b}[0;36m-\u{1b}[0mwhatever context\n",
            |d, o| d.print_context(o, "fnY", 1233, line),
        );
        check_display(
            Display::new().show_filename(true).show_lineno(true),
            "\u{1b}[0;35mfnZ\u{1b}[0;36m-\u{1b}[0;32m1234\
             \u{1b}[0;36m-\u{1b}[0mwhatever context\n",
            |d, o| d.print_context(o, "fnZ", 1234, line),
        );
    }

    #[test]
    fn display_print_context_delimiter() {
        const DELIM: &str = "\u{1b}[0;36m--\n\u{1b}[0m";
        check_display(Display::new(), DELIM, |d, o| {
            d.print_context_delimiter(o, "unused0", 11)
        });
        check_display(Display::new().show_filename(true), DELIM, |d, o| {
            d.print_context_delimiter(o, "unused1", 12)
        });
        check_display(Display::new().show_lineno(true), DELIM, |d, o| {
            d.print_context_delimiter(o, "unused2", 13)
        });
        check_display(
            Display::new().show_filename(true).show_lineno(true),
            DELIM,
            |d, o| d.print_context_delimiter(o, "unused3", 14),
        );
    }

    #[test]
    fn display_print_line() {
        let line = b"/::ffff.1.2.3.4/255.255.0.0/\n";
        let matches = vec![
            NetCandidate {
                range: (1, 15),
                net: Net::from_str_unchecked("::ffff:1.2.3.4"),
            },
            NetCandidate {
                range: (16, 27),
                net: Net::from_str_unchecked("255.255.0.0"),
            },
        ];
        check_display(
            Display::new(),
            "/\u{1b}[1;31m::ffff.1.2.3.4\u{1b}[0m\
             /\u{1b}[1;31m255.255.0.0\u{1b}[0m/\n",
            |d, o| d.print_line(o, "fn", 1231, line, &matches),
        );
        check_display(
            Display::new().show_filename(true),
            "\u{1b}[0;35msome_fn\u{1b}[0;36m:\u{1b}[0m\
             /\u{1b}[1;31m::ffff.1.2.3.4\u{1b}[0m\
             /\u{1b}[1;31m255.255.0.0\u{1b}[0m/\n",
            |d, o| d.print_line(o, "some_fn", 1232, line, &matches),
        );
        check_display(
            Display::new().show_lineno(true),
            "\u{1b}[0;32m1233\u{1b}[0;36m:\u{1b}[0m\
             /\u{1b}[1;31m::ffff.1.2.3.4\u{1b}[0m\
             /\u{1b}[1;31m255.255.0.0\u{1b}[0m/\n",
            |d, o| d.print_line(o, "some_fn", 1233, line, &matches),
        );
        check_display(
            Display::new().show_filename(true).show_lineno(true),
            "\u{1b}[0;35msome_fn\u{1b}[0;36m:\
             \u{1b}[0;32m1234\u{1b}[0;36m:\u{1b}[0m\
             /\u{1b}[1;31m::ffff.1.2.3.4\u{1b}[0m\
             /\u{1b}[1;31m255.255.0.0\u{1b}[0m/\n",
            |d, o| d.print_line(o, "some_fn", 1234, line, &matches),
        );
    }
}
