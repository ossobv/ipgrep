use std::fmt;

use crate::net::{IpNet, Net};

#[derive(Clone, Copy, Debug, Default)]
pub struct AcceptSet {
    pub ip: bool,
    pub net: bool,
    pub oldnet: bool,
    pub iface: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum InterfaceMode {
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
    // FIXME(future): touches? 10.0.10.0/24 <-> 10.0.11.0/24
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
                haystack.0.contains(&needle.0.network())
                    && haystack.0.contains(&needle.0.broadcast())
            }

            MatchMode::Within => {
                needle.0.contains(&haystack.0.network())
                    && needle.0.contains(&haystack.0.broadcast())
            }

            MatchMode::Overlaps => matchmode_overlaps(haystack, needle),
        }
    }
}

// Helper to determine overlap
fn matchmode_overlaps(a: &Net, b: &Net) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn check(mode: MatchMode, cases: &[(&str, &str, bool)]) {
        for &(haystack, needle, expected) in cases {
            let h = Net::try_from(haystack).unwrap();
            let n = Net::try_from(needle).unwrap();
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
    fn ipv4_vs_ipv6_is_false() {
        // FIXME: match ::ffff:10.0.0.0/8 maybe?
        let a = Net::try_from("10.0.0.0/8").unwrap();
        let b = Net::try_from("2001:db8::/32").unwrap();
        assert!(!MatchMode::Overlaps.matches(&a, &b));
        assert!(!MatchMode::Contains.matches(&a, &b));
        assert!(!MatchMode::Within.matches(&a, &b));
        assert!(!MatchMode::Equals.matches(&a, &b));
    }

    #[test]
    fn equals_v6() {
        let a = Net::try_from("2001:db8::/32").unwrap();
        let b = Net::try_from("2001:db8::/32").unwrap();
        assert!(MatchMode::Equals.matches(&a, &b));
    }

    #[test]
    fn contains_v6() {
        let hay = Net::try_from("2001:db8::/32").unwrap();
        let needle = Net::try_from("2001:db8:1::/48").unwrap();
        assert!(MatchMode::Contains.matches(&hay, &needle));
    }
}
