use std::str::FromStr;

use clap::error::ErrorKind;
use clap::{ArgAction, Error, Parser, ValueEnum};

use crate::params::{
    AcceptSet, Context, InterfaceMode, MatchMode, Needle, OutputStyle,
    Parameters, RecurseHaystacks,
};

#[derive(Clone, ValueEnum, Debug)]
pub enum AcceptSetArg {
    // no alias, "ip" is default and short enough
    Ip,
    #[value(alias = "n")]
    Net,
    #[value(alias = "o")]
    Oldnet,
    #[value(alias = "if")]
    Iface,
}

#[derive(Clone, ValueEnum, Debug)]
pub enum InterfaceModeArg {
    // no alias, "ip" is default and short enough
    Ip,
    #[value(alias = "n")]
    Net,
    #[value(alias = "c")]
    Complain,
}

#[derive(Clone, ValueEnum, Debug)]
pub enum MatchModeArg {
    #[value(alias = "c")]
    Contains,
    #[value(alias = "w")]
    Within,
    #[value(alias = "e")]
    Equals,
    #[value(alias = "o")]
    Overlaps,
}

#[derive(Clone, Debug)]
pub struct NeedleArg(pub String);

#[derive(Parser, Debug)]
#[command(
    name="ipgrep",
    version,
    about="Search IP addresses and networks in text files",
    disable_help_flag=true,     // we use "-h" for "no-filename"
    disable_version_flag=true,  // we position it manually
    before_help="",
    after_help="\
Options mimic classic grep options.

Exit status:
  0 if match found
  1 if no match found
  2 if error

Example invocations:
  # Look for a few IPs in all networks found in /etc.
  ipgrep -C 5 -a net -a oldnet -r 192.168.2.5,192.168.2.78 /etc/*

  # Output linefeed separated IPs of all IPv4 hosts/interfaces.
  ipgrep -m within -o 0.0.0.0/0 input.txt",
    help_template="\
{name} {version} - {about}

{usage-heading} {usage}{before-help}{all-args}{after-help}"
)]
pub struct Args {
    /// Accept input forms (may repeat)
    #[arg(
        short='a', long="accept", value_enum,
        default_values_t=vec![
            AcceptSetArg::Ip, AcceptSetArg::Net, AcceptSetArg::Iface],
        help_heading="Matching Control",
        long_help="\
Accept input forms (may repeat or use commas):
  ip        - bare host IP
  net       - valid network (CIDR)
  oldnet    - valid network (host/dotted-netmask)
  iface     - interface IP (host/mask)"
    )]
    pub accept: Vec<AcceptSetArg>,

    /// Select interface IP matching mode
    #[arg(
        short='I', long="interface-mode", value_enum,
        default_value_t=InterfaceModeArg::Ip,
        help_heading="Matching Control",
        long_help="\
Select interface IP matching mode:
  ip        - treat as single IP
  net       - treat as if network bits were unset
  complain  - complain/reject when network bits are set"
    )]
    pub interface_mode: InterfaceModeArg,

    /// Match mode
    #[arg(
        short='m', long="match", value_enum,
        default_value_t=MatchModeArg::Contains,
        help_heading="Matching Control",
        long_help="\
Match mode:
   contains - haystack net contains needle net
   within   - haystack net is within needle net (inverse of contains)
   equals   - exact IP or network equality
   overlaps - haystack and needle nets overlap"
    )]
    pub match_mode: MatchModeArg,

    /// Print only a count of matching records
    #[arg(
        short = 'c',
        long = "count",
        help_heading = "General Output Control"
    )]
    pub count: bool,

    /// List filenames with matches only
    #[arg(
        short = 'l',
        long = "files-with-matches",
        help_heading = "General Output Control"
    )]
    pub files_with_matches: bool,

    /// Print only the matching IPs/networks
    #[arg(
        short = 'o',
        long = "only-matching",
        help_heading = "General Output Control"
    )]
    pub only_matching: bool,

    /// Quiet; exit status only
    #[arg(
        short = 'q',
        long = "quiet",
        help_heading = "General Output Control"
    )]
    pub quiet: bool,

    /// Suppress filename prefix on output
    #[arg(
        short = 'h',
        long = "no-filename",
        help_heading = "Output Line Prefix Control"
    )]
    pub no_filename: bool,

    /// Prefix each output line/record with lineno
    #[arg(
        short = 'n',
        long = "line-number",
        help_heading = "Output Line Prefix Control"
    )]
    pub line_number: bool,

    /// Output a zero byte instead of LF in output; only useful in
    /// combination with -l
    #[arg(
        short = 'Z',
        long = "null",
        help_heading = "Output Line Prefix Control"
    )]
    pub null: bool,

    /// print NUM lines of leading context
    #[arg(
        short = 'B',
        long = "before-context",
        help_heading = "Context Line Control"
    )]
    pub before_context: Option<usize>,

    /// print NUM lines of trailing context
    #[arg(
        short = 'A',
        long = "after-context",
        help_heading = "Context Line Control"
    )]
    pub after_context: Option<usize>,

    /// print NUM lines of output context
    #[arg(
        short = 'C',
        long = "context",
        help_heading = "Context Line Control"
    )]
    pub context: Option<usize>,

    /// Read all files under each directory, recursively
    #[arg(
        short = 'r',
        long = "recursive",
        help_heading = "File and Directory Selection"
    )]
    pub recursive: bool,

    /// Read all files under each directory, while dereferencing
    /// symlinks to directories
    #[arg(
        short = 'R',
        long = "dereference-recursive",
        help_heading = "File and Directory Selection"
    )]
    pub deref_recursive: bool,

    /// Flush output on every line
    #[arg(long = "line-buffered", help_heading = "Other Options")]
    pub line_buffered: bool,

    /// Show help
    #[arg(
        long="help", action = ArgAction::Help,
        help_heading="Generic Program Information"
    )]
    pub help: Option<bool>,

    /// Show program version
    #[arg(short = 'V', long="version", action = ArgAction::Version,
        help_heading="Generic Program Information",
    )]
    pub version: Option<bool>,

    /// Needles (one or more networks separated by comma or whitespace)
    ///
    /// For example: 192.168.2.0/24,2001:db8::/32
    pub needles: NeedleArg,

    /// Haystacks are one or more files. If none (or '-') given, stdin is read.
    pub haystacks: Vec<String>,
}

const ERR_CONTEXT_CONFLICT: &str = "\
--context conflicts with --before-context/--after-context\n";
const ERR_RECURSIVE_CONFLICT: &str = "\
choose either --recursive or --deref-recursive\n";

impl Args {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }

    /// Convert CLI args to internal parameters
    pub fn into_parameters(self) -> Parameters {
        // Take these first before we make self partial.
        let context = self.make_context();
        let recursive = self.make_recursive();
        let output_style = self.make_output_style();

        Parameters {
            accept: self.accept.into(),
            interface_mode: self.interface_mode.into(),
            match_mode: self.match_mode.into(),
            output_style,
            hide_filename: self.no_filename,
            show_lineno: self.line_number,
            show_context: context,
            recursive,
            line_buffered: self.line_buffered,
            needles: self.needles.into(),
            haystack_filenames: self.haystacks,
        }
    }

    fn make_context(&self) -> Context {
        let mut context = Context::default();
        if let Some(value) = self.context {
            if self.before_context.is_some() || self.after_context.is_some() {
                Error::raw(ErrorKind::ArgumentConflict, ERR_CONTEXT_CONFLICT)
                    .exit();
            } else {
                context.before = value;
                context.after = value;
            }
        } else {
            if let Some(value) = self.before_context {
                context.before = value;
            }
            if let Some(value) = self.after_context {
                context.after = value;
            }
        }
        context
    }

    fn make_recursive(&self) -> RecurseHaystacks {
        if self.deref_recursive && self.recursive {
            Error::raw(ErrorKind::ArgumentConflict, ERR_RECURSIVE_CONFLICT)
                .exit();
        }
        if self.deref_recursive {
            RecurseHaystacks::FollowDirectorySymlinks
        } else if self.recursive {
            RecurseHaystacks::FollowDirectories
        } else {
            RecurseHaystacks::No
        }
    }

    // GNU grep (3.11) has these output modes:
    // "-q/--quiet" shows nothing;
    // "-l/--files-with-matches" only shows files;
    // "-c/--count" shows files with counts (not a grand total);
    // "-o/--only-matching" shows the matches;
    // -q trumps -l, -l trumps -c, -c trumps -o.
    fn make_output_style(&self) -> OutputStyle {
        if self.quiet {
            // -q/--quiet
            OutputStyle::ShowNothing
        } else if self.files_with_matches && self.null {
            // -l/--file-with-matches, -Z/--null
            OutputStyle::ShowFilesOnlyNull
        } else if self.files_with_matches {
            // -l/--file-with-matches
            OutputStyle::ShowFilesOnly
        } else if self.count {
            // -c/--count
            OutputStyle::ShowCountsPerFile
        } else if self.only_matching {
            // -o/--only-matching
            OutputStyle::ShowOnlyMatching
        } else {
            OutputStyle::ShowLinesAndContext
        }
    }

    /*
    fn make_show_filename(&self) -> bool {
        // Show filenames if haystacks > 1 ..
        if self.haystacks.len() > 1 {
            !self.no_filename // .. except if the user wants them hidden
        } else {
            false
        }
    }
    */
}

/// Conversion from String to NeedleArg during clap arg parsing
impl FromStr for NeedleArg {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(NeedleArg(s.to_string()))
    }
}

/// Conversion helper for AcceptSetArg to AcceptSet
impl From<Vec<AcceptSetArg>> for AcceptSet {
    fn from(args: Vec<AcceptSetArg>) -> Self {
        let mut set = AcceptSet::default();
        for arg in args {
            match arg {
                AcceptSetArg::Ip => set.ip = true,
                AcceptSetArg::Net => set.net = true,
                AcceptSetArg::Oldnet => set.oldnet = true,
                AcceptSetArg::Iface => set.iface = true,
            }
        }
        set
    }
}

/// Conversion helper for InterfaceModeArg to InterfaceMode
impl From<InterfaceModeArg> for InterfaceMode {
    fn from(i: InterfaceModeArg) -> Self {
        match i {
            InterfaceModeArg::Ip => InterfaceMode::TreatAsIp,
            InterfaceModeArg::Net => InterfaceMode::TreatAsNetwork,
            InterfaceModeArg::Complain => InterfaceMode::ComplainAndSkip,
        }
    }
}

/// Conversion helper for MatchModeArg to MatchMode
impl From<MatchModeArg> for MatchMode {
    fn from(m: MatchModeArg) -> Self {
        match m {
            MatchModeArg::Contains => MatchMode::Contains,
            MatchModeArg::Within => MatchMode::Within,
            MatchModeArg::Equals => MatchMode::Equals,
            MatchModeArg::Overlaps => MatchMode::Overlaps,
        }
    }
}

/// Conversion helper for NeedleArg to Needle
impl From<NeedleArg> for Vec<Needle> {
    fn from(s: NeedleArg) -> Vec<Needle> {
        let mut needles = Vec::new();
        for tok in s.0.split([',', ' ']) {
            let trimmed = tok.trim();
            if trimmed.is_empty() {
                continue;
            }
            match Needle::try_from(tok) {
                Ok(needle) => needles.push(needle),
                Err(err) => {
                    Error::raw(ErrorKind::InvalidValue, format!("{err}\n"))
                        .exit();
                }
            }
        }
        needles
    }
}
