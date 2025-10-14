use std::io::Write;

use crate::scanner::NetCandidate;

const COLOR_MATCH: &str = "\x1b[1;31m"; // red <results>
const COLOR_LINENO: &str = "\x1b[0;32m"; // green <number>
const COLOR_DELIMITER: &str = "\x1b[0;36m"; // blue ":"/"-"
const COLOR_FILENAME: &str = "\x1b[0;35m"; // purple <filename>
const COLOR_RESET: &str = "\x1b[0m";

/// Output modes, in order of precedence
#[derive(Debug)]
pub enum OutputStyle {
    // Only exit with status
    ShowNothing,
    // Only files that have a match, NUL terminated
    ShowFilesOnlyNull,
    // Only files that have a match
    ShowFilesOnly,
    // All files, and a count of matches
    ShowCountsPerFile,
    // Show only the matches (no lines, no context)
    ShowOnlyMatching,
    // Show the lines (and optional context)
    ShowLinesAndContext,
}

pub struct Display {
    show_filename: bool,
    show_line_number: bool,
    with_color: bool,
}

impl Display {
    pub fn new(
        show_filename: bool,
        show_line_number: bool,
        with_color: bool,
    ) -> Self {
        // FIXME: change to builder pattern
        Self {
            show_filename,
            show_line_number,
            with_color,
        }
    }

    pub fn print_filename(
        &self,
        writer: &mut dyn Write,
        filename: &str,
        end: &[u8],
    ) -> std::io::Result<()> {
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
    ) -> std::io::Result<()> {
        if self.show_filename {
            self.write_filename(writer, filename)?;
            self.write_delimiter(writer, b":")?;
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
    ) -> std::io::Result<()> {
        for match_ in matches {
            if self.show_filename {
                self.write_filename(writer, filename)?;
                self.write_delimiter(writer, b":")?;
            }
            if self.show_line_number {
                self.write_linenumber(writer, lineno)?;
                self.write_delimiter(writer, b":")?;
            }
            self.write_match(writer, line, match_)?;
            self.write_no_color(writer)?;
            self.write(writer, b"\n")?;
        }
        Ok(())
    }

    pub fn print_line(
        &self,
        writer: &mut dyn Write,
        filename: &str,
        lineno: usize,
        line: &[u8],
        matches: &Vec<NetCandidate>,
    ) -> std::io::Result<()> {
        if self.show_filename {
            self.write_filename(writer, filename)?;
            self.write_delimiter(writer, b":")?;
            self.write_no_color(writer)?;
        }
        if self.show_line_number {
            self.write_linenumber(writer, lineno)?;
            self.write_delimiter(writer, b":")?;
            self.write_no_color(writer)?;
        }
        self.write_line(writer, line, matches)?;
        Ok(())
    }

    #[inline]
    fn write(
        &self,
        writer: &mut dyn Write,
        value: &[u8],
    ) -> std::io::Result<()> {
        writer.write_all(value)?;
        Ok(())
    }

    #[inline]
    fn write_no_color(&self, writer: &mut dyn Write) -> std::io::Result<()> {
        if self.with_color {
            writer.write_all(COLOR_RESET.as_bytes())?;
        }
        Ok(())
    }

    #[inline]
    fn write_count(
        &self,
        writer: &mut dyn Write,
        count: usize,
    ) -> std::io::Result<()> {
        writer.write_all(format!("{count}\n").as_bytes())?;
        Ok(())
    }

    #[inline]
    fn write_delimiter(
        &self,
        writer: &mut dyn Write,
        delim: &[u8],
    ) -> std::io::Result<()> {
        if self.with_color {
            writer.write_all(COLOR_DELIMITER.as_bytes())?;
        }
        writer.write_all(delim)?;
        Ok(())
    }

    #[inline]
    fn write_filename(
        &self,
        writer: &mut dyn Write,
        filename: &str,
    ) -> std::io::Result<()> {
        if self.with_color {
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
    ) -> std::io::Result<()> {
        if self.with_color {
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
    ) -> std::io::Result<()> {
        if self.with_color {
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
    ) -> std::io::Result<()> {
        if self.with_color {
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
