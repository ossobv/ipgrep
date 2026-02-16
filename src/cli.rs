use std::str::FromStr;

use clap::error::ErrorKind;
use clap::{ArgAction, Error, Parser, ValueEnum, value_parser};

use crate::params::{
    AcceptSet, InterfaceMode, MatchMode, Needle, OutputStyle, Parameters,
    RecurseHaystacks, ShowContext,
};

#[cfg(feature = "version-from-env")]
const GIT_VERSION: &str = env!("GIT_VERSION");
#[cfg(not(feature = "version-from-env"))]
const GIT_VERSION: &str = git_version::git_version!();

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
    #[value(alias = "a")]
    Auto,
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
    version=GIT_VERSION,
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
  ipgrep -C 5 -a net -a oldnet [-m contains] -r 192.168.2.5,10.0.2.1 /etc/*

  # Output linefeed separated IPs of all IPv4 hosts/interfaces.
  ipgrep [-m within] -o 0.0.0.0/0 input.txt

  # Find all unique /24 networks in a pcap. Requires a bit of sed magic
  # because tcpdump outputs the port as the fifth octet.
  tcpdump -nr my.pcap |
    sed -Ee 's/([0-9]+([.][0-9]+){3})[.]([0-9]+)/\\1:\\3/g' |
    ipgrep -O24 | sort | uniq -c | sort -k2V
  ",
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
        default_value_t=MatchModeArg::Auto,
        help_heading="Matching Control",
        long_help="\
Match mode:
   auto     - 'contains' if the all needles are a single IP, else 'within'
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

    /// Print only the matching IPs/networks, but changed to the
    /// specified network size
    #[arg(
        short = 'O',
        long = "output-prefix",
        help_heading = "General Output Control",
        long_help="\
Implies -o/--only-matching. Truncates found IPs/networks to the specified
prefix length. E.g. pass 24 to get 192.168.2.0/24 instead of 192.168.2.4",
        value_parser = value_parser!(u8).range(0..=128)
    )]
    pub output_prefix: Option<u8>,

    /// Quiet; exit status only
    #[arg(
        short = 'q',
        long = "quiet",
        help_heading = "General Output Control"
    )]
    pub quiet: bool,

    /// Select non-matching lines, will include non-IPs in output
    #[arg(
        short = 'v',
        long = "invert-match",
        help_heading = "General Output Control"
    )]
    pub invert_match: bool,

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
    /// tandem with -l
    #[arg(
        short = 'Z',
        long = "null",
        help_heading = "Output Line Prefix Control"
    )]
    pub null: bool,

    /// Print NUM lines of leading context
    #[arg(
        short = 'B',
        long = "before-context",
        help_heading = "Context Line Control"
    )]
    pub before_context: Option<usize>,

    /// Print NUM lines of trailing context
    #[arg(
        short = 'A',
        long = "after-context",
        help_heading = "Context Line Control"
    )]
    pub after_context: Option<usize>,

    /// Print NUM lines of output context
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
    /// found symlinks
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
    #[arg(
        default_value = "ip4,ip6",
        long_help = "\
Needles are one or more IP addresses, networks or IP classes (separated
by comma/whitespace). Needles may be negated by prefixing them with a '!'.
Matches are made if any of the positive needles match and none of the
negative ones do.

Examples of valid needles:
- ip4,ip6 (default)
- 192.168.0.0/16
- 10.0.0.0/8,!10.2.0.0/16,fc00::/7
- ip4,!rfc1918

Valid classes include: ip4, ip6, global, localhost4, multicast6, private."
    )]
    pub needles: NeedleArg,

    /// Haystacks are one or more files. If none (or '-') given, stdin is read.
    pub haystacks: Vec<String>,
}

const ERR_CONTEXT_CONFLICT: &str = "\
--context conflicts with --before-context/--after-context\n";
const ERR_INVONLY_CONFLICT: &str = "\
--invert-match conflicts with --only-matching/--output-prefix\n";
const ERR_RECURSIVE_CONFLICT: &str = "\
choose either --recursive or --deref-recursive\n";

impl Args {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }

    /// Convert CLI args to internal parameters
    pub fn into_parameters(self) -> Parameters {
        // Take these first before we make self partial.
        let output_style = self.make_output_style();
        let show_context = self.make_show_context();
        let recursive = self.make_recursive();

        let all_needles: Vec<Needle> = self.needles.into();

        // Match mode depends on the needles.
        let match_mode: MatchMode = self.match_mode.resolve(&all_needles);

        // Needles are split into positive and negative ones.
        let (negative_needles, mut positive_needles): (
            Vec<Needle>,
            Vec<Needle>,
        ) = all_needles.into_iter().partition(|n| n.is_negated);
        if positive_needles.is_empty() {
            // Design choice: if the user specifies "!rfc1918" they will
            // only get IPv4 addresses.  If they want IPv6 as well, they
            // should use "any,!rfc1918".
            let has_v4 = negative_needles.iter().any(|n| n.net.is_ipv4());
            let has_v6 = negative_needles.iter().any(|n| n.net.is_ipv6());
            assert!(has_v4 || has_v6);
            if has_v4 {
                positive_needles.push(Needle::try_from("0.0.0.0/0").unwrap());
            }
            if has_v6 {
                positive_needles.push(Needle::try_from("::/0").unwrap());
            }
        }

        Parameters {
            accept: self.accept.into(),
            interface_mode: self.interface_mode.into(),
            match_mode,
            output_style,
            rewrite_output_prefix: self.output_prefix,
            invert_match: self.invert_match,
            hide_filename: self.no_filename,
            show_lineno: self.line_number,
            show_context,
            recursive,
            line_buffered: self.line_buffered,
            positive_needles,
            negative_needles,
            haystack_filenames: self.haystacks,
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
            OutputStyle::JustExitCode
        } else if self.files_with_matches && self.null {
            // -l/--file-with-matches, -Z/--null
            OutputStyle::ShowFilesWithNull
        } else if self.files_with_matches {
            // -l/--file-with-matches
            OutputStyle::ShowFilesWithLf
        } else if self.count {
            // -c/--count
            OutputStyle::ShowCountsPerFile
        } else if self.only_matching || self.output_prefix.is_some() {
            // -o/--only-matching
            if self.invert_match {
                Error::raw(ErrorKind::ArgumentConflict, ERR_INVONLY_CONFLICT)
                    .exit();
            }
            OutputStyle::ShowOnlyMatching
        } else {
            OutputStyle::ShowLinesAndContext
        }
    }

    fn make_show_context(&self) -> ShowContext {
        let mut context = ShowContext::default();
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
impl MatchModeArg {
    /// Resolves the CLI argument into a concrete MatchMode,
    /// using the parsed needles to determine the behavior of 'Auto'.
    pub fn resolve(self, needles: &[Needle]) -> MatchMode {
        match self {
            MatchModeArg::Auto => {
                // For Auto mode, we consider all needles:
                // - is any larger than a single IP? then Within
                // - else? Contains
                let all_are_single_ip =
                    needles.iter().all(|n| n.net.is_single_ip());

                if all_are_single_ip {
                    MatchMode::Contains
                } else {
                    MatchMode::Within
                }
            }
            MatchModeArg::Contains => MatchMode::Contains,
            MatchModeArg::Within => MatchMode::Within,
            MatchModeArg::Equals => MatchMode::Equals,
            MatchModeArg::Overlaps => MatchMode::Overlaps,
        }
    }
}

/// Conversion helper for NeedleArg to Vec<Needle>
impl From<NeedleArg> for Vec<Needle> {
    fn from(s: NeedleArg) -> Vec<Needle> {
        let mut needles = Vec::new();
        for tok in s.0.split([',', ' ']) {
            let trimmed = tok.trim();
            if trimmed.is_empty() {
                continue;
            }

            match Needle::parse(trimmed) {
                Ok(parsed_needles) => needles.extend(parsed_needles),
                Err(err) => {
                    Error::raw(ErrorKind::InvalidValue, format!("{err}\n"))
                        .exit();
                }
            }
        }

        if needles.is_empty() {
            // Default to 'any'.
            needles = Needle::parse("any").unwrap();
        }
        needles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_mode_auto_contains_because_no_needles() {
        // No needles => contains
        let needles: Vec<Needle> = vec![];
        let match_mode = MatchModeArg::Auto.resolve(&needles);
        assert!(matches!(match_mode, MatchMode::Contains));
    }

    #[test]
    fn test_match_mode_auto_contains_because_single_ip() {
        let needles: Vec<Needle> =
            vec![Needle::try_from("123.123.123.123").unwrap()];
        let match_mode = MatchModeArg::Auto.resolve(&needles);
        assert!(matches!(match_mode, MatchMode::Contains));
    }

    #[test]
    fn test_match_mode_auto_contains_because_multiple_ips() {
        let needles: Vec<Needle> = vec![
            Needle::try_from("1.2.3.4").unwrap(),
            Needle::try_from("fe01::123/128").unwrap(),
        ];
        let match_mode = MatchModeArg::Auto.resolve(&needles);
        assert!(matches!(match_mode, MatchMode::Contains));
    }

    #[test]
    fn test_match_mode_auto_within_because_single_net() {
        let needles: Vec<Needle> =
            vec![Needle::try_from("192.168.0.0/16").unwrap()];
        let match_mode = MatchModeArg::Auto.resolve(&needles);
        assert!(matches!(match_mode, MatchMode::Within));
    }

    #[test]
    fn test_match_mode_auto_within_because_first_is_net() {
        let needles: Vec<Needle> = vec![
            Needle::try_from("192.168.0.0/16").unwrap(),
            Needle::try_from("::1/128").unwrap(),
        ];
        let match_mode = MatchModeArg::Auto.resolve(&needles);
        assert!(matches!(match_mode, MatchMode::Within));
    }

    #[test]
    fn test_match_mode_auto_within_because_second_is_net() {
        let needles: Vec<Needle> = vec![
            Needle::try_from("1.2.3.4").unwrap(),
            Needle::try_from("::2/127").unwrap(),
        ];
        let match_mode = MatchModeArg::Auto.resolve(&needles);
        assert!(matches!(match_mode, MatchMode::Within));
    }
}
