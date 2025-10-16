use std::fmt;

use crate::net::{IpNet, Net};

#[derive(Clone, Copy, Debug, Default)]
pub struct AcceptSet {
    pub ip: bool,
    pub net: bool,
    pub oldnet: bool,
    pub iface: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum InterfaceMode {
    #[default]
    TreatAsIp,
    TreatAsNetwork,
    ComplainAndSkip,
}

#[derive(Debug)]
pub enum MatchMode {
    Equals,
    Contains,
    Within,
    Overlaps,
    // TODO: Does anyone want "Touches" for 10.0.10.0/24 <-> 10.0.11.0/24?
}

impl fmt::Display for MatchMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = match self {
            MatchMode::Equals => "equals",
            MatchMode::Contains => "contains",
            MatchMode::Within => "within",
            MatchMode::Overlaps => "overlaps",
        };
        write!(f, "{}", val)
    }
}

impl MatchMode {
    pub fn matches(&self, haystack: &Net, needle: &Net) -> bool {
        match self {
            MatchMode::Equals => needle.0 == haystack.0,

            MatchMode::Contains => {
                assert!(!needle.has_host_bits(), "{needle} has host bits");
                haystack.0.contains(&needle.0.network())
                    && haystack.0.contains(&needle.0.broadcast())
            }

            MatchMode::Within => {
                assert!(!haystack.has_host_bits(), "{haystack} has host bits");
                needle.0.contains(&haystack.0.network())
                    && needle.0.contains(&haystack.0.broadcast())
            }

            MatchMode::Overlaps => Self::overlaps(haystack, needle),
        }
    }

    // Helper to determine overlap
    fn overlaps(a: &Net, b: &Net) -> bool {
        match (a.0, b.0) {
            (IpNet::V4(a4), IpNet::V4(b4)) => {
                a4.contains(&b4.network()) || b4.contains(&a4.network())
            }
            (IpNet::V6(a6), IpNet::V6(b6)) => {
                a6.contains(&b6.network()) || b6.contains(&a6.network())
            }
            // TODO: do we need to do some ::ffff.1.2.3.4 IPv4 mapping checks?
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(mode: MatchMode, cases: &[(&str, &str, bool)]) {
        for &(haystack, needle, expected) in cases {
            let h = Net::from_str_unchecked(haystack);
            let n = Net::from_str_unchecked(needle);
            let got = mode.matches(&h, &n);
            let hint = match expected {
                true => "should match",
                false => "should not match",
            };
            assert_eq!(
                got, expected,
                "haystack={haystack} match {mode} needle={needle}; {hint}"
            );
        }
    }

    #[test]
    fn equals_v4() {
        check(
            MatchMode::Equals,
            &[
                ("10.0.0.0/24", "10.0.0.0/24", true),
                ("10.0.0.0/24", "10.0.1.0/24", false),
                ("192.168.0.0/16", "192.168.0.0/16", true),
                ("192.168.0.0/16", "192.168.1.0/24", false),
                ("1.2.3.4", "1.2.3.4", true),
                ("1.2.3.4", "1.2.3.5", false),
            ],
        );
    }

    #[test]
    fn equals_v6() {
        check(
            MatchMode::Equals,
            &[
                ("::1/8", "0::1/8", true),
                ("::1/128", "0:0:0:0:0:0:0:1/128", true),
                ("::1/128", "0:0:0:0:0:0:0:1/127", false),
                ("2001:db8::/32", "2001:db8::/32", true),
                ("2001:db8::/64", "2001:db8::/64", true),
            ],
        );
    }

    #[test]
    fn contains_v4() {
        check(
            MatchMode::Contains,
            &[
                ("10.0.0.0/24", "10.0.0.0/24", true),
                ("10.0.0.0/24", "10.0.0.0/23", false),
                ("10.0.0.0/23", "10.0.0.0/24", true),
                ("10.0.0.128/25", "10.0.0.0/24", false),
                ("192.168.0.0/16", "192.168.0.0/16", true),
                ("192.168.3.3", "192.168.3.0/30", false),
                ("1.2.3.4", "1.2.3.4", true),
                ("1.2.3.4", "1.2.3.5", false),
            ],
        );
    }

    #[test]
    fn contains_v6() {
        check(
            MatchMode::Contains,
            &[
                ("2001:db8:1::/32", "2001:db8::/33", true),
                ("2001:db8:1::/32", "2001:db8::/32", true),
                ("2001:db8:1::/32", "2001:db8::/31", false),
            ],
        );
    }

    #[test]
    fn within_v4() {
        check(
            MatchMode::Within,
            &[
                ("10.0.0.0/24", "10.0.0.0/24", true),
                ("10.0.0.0/24", "10.0.0.0/23", true),
                ("10.0.0.0/23", "10.0.0.0/24", false),
                ("10.0.0.128/25", "10.0.0.0/24", true),
                ("192.168.0.0/16", "192.168.0.0/16", true),
                ("192.168.3.3", "192.168.3.0/30", true),
                ("1.2.3.4", "1.2.3.4", true),
                ("1.2.3.4", "1.2.3.5", false),
            ],
        );
    }

    #[test]
    fn within_v6() {
        check(
            MatchMode::Within,
            &[
                ("dead:beef:c0ff:ee00:dead:beef:c0ff:ee00", "::", false),
                ("dead:beef:c0ff:ee00:dead:beef:c0ff:ee00", "::/0", true),
                ("2001:db8:1::/50", "2001:db8:1::/49", true),
                ("2001:db8:1::/49", "2001:db8:1::/49", true),
                ("2001:db8:1::/48", "2001:db8:1::/49", false),
            ],
        );
    }

    #[test]
    fn overlaps_v4() {
        check(
            MatchMode::Overlaps,
            &[
                ("10.0.0.0/24", "10.0.0.0/24", true),
                ("10.0.0.0/24", "10.0.0.0/23", true),
                ("10.0.0.0/23", "10.0.0.0/24", true),
                ("10.0.0.128/25", "10.0.0.0/24", true),
                ("10.0.0.0/24", "10.0.0.128/25", true),
                ("1.2.3.4", "1.2.3.4", true),
                ("1.2.3.4", "1.2.3.5", false),
                ("1.2.3.0/24", "1.2.4.0/24", false),
            ],
        );
    }

    #[test]
    fn overlaps_v6() {
        check(
            MatchMode::Overlaps,
            &[
                ("::1", "::2", false),
                ("2001:db8:1::/33", "2001:db8::/32", true),
                ("2001:db8:1::/32", "2001:db8::/32", true),
                ("2001:db8:1::/31", "2001:db8::/32", true),
            ],
        );
    }

    #[test]
    fn ipv4_vs_ipv6_is_false() {
        let a = Net::from_str_unchecked("10.0.0.0/8");
        let b = Net::from_str_unchecked("2001:db8::/32");
        assert!(!MatchMode::Overlaps.matches(&a, &b));
        assert!(!MatchMode::Contains.matches(&a, &b));
        assert!(!MatchMode::Within.matches(&a, &b));
        assert!(!MatchMode::Equals.matches(&a, &b));
    }

    /// Disabled test that expects "::ffff:0:0/96" (v6) to equal "0.0.0.0/0".
    /// Not sure if we want to keep this around.
    #[test]
    #[should_panic(expected = "\
        assertion failed: MatchMode::Overlaps.matches(&a, &b)")]
    fn ipv4_vs_ipv6_is_true() {
        let a = Net::from_str_unchecked("10.0.0.0/8");
        let b = Net::from_str_unchecked("::ffff:10.0.0.0/104");
        assert!(MatchMode::Overlaps.matches(&a, &b));
        assert!(MatchMode::Contains.matches(&a, &b));
        assert!(MatchMode::Within.matches(&a, &b));
        assert!(MatchMode::Equals.matches(&a, &b));
    }
}
