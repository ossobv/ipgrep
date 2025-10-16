use memchr::{memchr_iter, memchr2_iter};

use crate::matching::{AcceptSet, InterfaceMode};
use crate::net::Net;
use crate::netlike::NetLikeScanner;

#[derive(Debug, PartialEq)]
pub struct NetCandidate {
    pub range: (usize, usize),
    pub net: Net,
}

pub struct NetCandidateScanner {
    // These could be relevant for the subparser:
    include_ipv4: bool,
    include_ipv6: bool,
    accept: AcceptSet,
    // These is first relevant here after we've found the matches:
    interface_mode: InterfaceMode,
}

impl NetCandidateScanner {
    pub fn new() -> Self {
        Self {
            include_ipv4: true,
            include_ipv6: true,
            accept: AcceptSet::default(),
            interface_mode: InterfaceMode::default(),
        }
    }

    pub fn ignore_ipv4(self, value: bool) -> Self {
        if value && !self.include_ipv6 {
            panic!("no IPv4 or IPv6 needles supplied");
        }
        Self {
            include_ipv4: !value,
            ..self
        }
    }
    pub fn ignore_ipv6(self, value: bool) -> Self {
        if value && !self.include_ipv4 {
            panic!("no IPv4 or IPv6 needles supplied");
        }
        Self {
            include_ipv6: !value,
            ..self
        }
    }

    pub fn set_accept(self, accept: AcceptSet) -> Self {
        assert!(accept.ip || accept.net || accept.oldnet || accept.iface);
        Self { accept, ..self }
    }

    pub fn set_interface_mode(self, interface_mode: InterfaceMode) -> Self {
        Self {
            interface_mode,
            ..self
        }
    }

    pub fn find_all(&self, buf: &[u8], filename: &str) -> Vec<NetCandidate> {
        let mut candidates = Vec::new();

        // This actually produces quite a speedup for the /etc/* dataset
        // of about 92ms to 40ms user time.
        if !match (self.include_ipv4, self.include_ipv6) {
            (true, true) => prefilter_could_be_ip(buf),
            (true, false) => prefilter_could_be_ip4(buf),
            (false, true) => prefilter_could_be_ip6(buf),
            (false, false) => unreachable!(),
        } {
            // The empty list.
            return candidates;
        }

        let netlikescanner = if self.accept.oldnet {
            NetLikeScanner::new(buf).with_oldnet()
        } else {
            NetLikeScanner::new(buf)
        };
        let nonet =
            !(self.accept.net || self.accept.oldnet || self.accept.iface);

        for (start, end) in netlikescanner {
            let mut slice = &buf[start..end];

            // Restrict based on IP or not-IP.
            match slice.iter().position(|&b| b == b'/') {
                Some(slash_pos) => {
                    // If there is a slash and we don't want networks.
                    // Go to IP mode immediately.
                    if nonet {
                        slice = &slice[0..slash_pos];
                    } else if self.accept.oldnet && !self.accept.net {
                        // iface without net normally implies net. If there
                        // is oldnet, we will only accept full old-style
                        // masks.
                        if !slice[slash_pos..].contains(&b'.') {
                            continue;
                        }
                    }
                }
                None => {
                    // There is no slash. Do we only want networks? Then skip.
                    if !self.accept.ip {
                        continue;
                    }
                }
            }

            let mut net = match Net::try_from(slice) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Do we reject or translate interfaces (networks with host bits)?
            if net.has_host_bits() {
                if !self.accept.iface {
                    continue;
                }
                // Translate/complain?
                net = match self.interface_mode {
                    InterfaceMode::TreatAsIp => net.as_ip(),
                    InterfaceMode::TreatAsNetwork => net.as_network(),
                    InterfaceMode::ComplainAndSkip => {
                        eprintln!(
                            "ipgrep: {filename}: warning: \
                             Ignoring network {net} with host bits set"
                        );
                        continue;
                    }
                }
            }

            // If we found an IP, check that we're doing Needle scans on those.
            if !self.include_ipv6 && net.is_ipv6() {
                // TODO: At one point, (re)consider whether we want to
                // treat "::ffff.1.2.3.4/96" as IPv4 space or not. For
                // now, we don't.
                continue;
            }
            if !self.include_ipv4 && net.is_ipv4() {
                continue;
            }

            // Found one.
            candidates.push(NetCandidate {
                range: (start, end),
                net,
            });
        }

        candidates
    }
}

/// The old regex implementation was rather slow. A prefilter reduced the times
/// from 150ms to 75ms (for the most basic regex). With the new advanced
/// iplikescanner, we can still benefit with a speedup from 100ms to 75ms.
/// Right now, this quickly checks for "[0-9][.][0-9]" and ":[0-9a-fA-F:]".
#[inline]
fn prefilter_could_be_ip(line: &[u8]) -> bool {
    // Check this, or we might fail at (line.len() - 1).
    if line.is_empty() {
        return false;
    }

    let maxpos = line.len() - 1;
    let it = memchr2_iter(b'.', b':', line);
    for pos in it {
        match line[pos] {
            // [0-9].[0-9] <-- could be IPv4
            b'.' => {
                if pos < maxpos
                    && line[pos + 1].is_ascii_digit()
                    && pos > 0
                    && line[pos - 1].is_ascii_digit()
                {
                    return true;
                }
            }
            // :[0-9a-fA-F:] <-- could be IPv6
            b':' => {
                if pos < maxpos
                    && (line[pos + 1].is_ascii_hexdigit()
                        || line[pos + 1] == b':')
                {
                    return true;
                }
            }
            _ => unreachable!(),
        }
    }
    false
}

#[inline]
fn prefilter_could_be_ip4(line: &[u8]) -> bool {
    // Check this, or we might fail at (line.len() - 1).
    if line.is_empty() {
        return false;
    }

    let maxpos = line.len() - 1;
    let it = memchr_iter(b'.', line);
    for pos in it {
        // [0-9].[0-9] <-- could be IPv4
        if pos < maxpos
            && line[pos + 1].is_ascii_digit()
            && pos > 0
            && line[pos - 1].is_ascii_digit()
        {
            return true;
        }
    }
    false
}

#[inline]
fn prefilter_could_be_ip6(line: &[u8]) -> bool {
    // Check this, or we might fail at (line.len() - 1).
    if line.is_empty() {
        return false;
    }

    let maxpos = line.len() - 1;
    let it = memchr_iter(b':', line);
    for pos in it {
        // :[0-9a-fA-F:] <-- could be IPv6
        if pos < maxpos
            && (line[pos + 1].is_ascii_hexdigit() || line[pos + 1] == b':')
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_mode_treat_as_ip() {
        let acc = AcceptSet {
            ip: true,
            net: true,
            oldnet: false,
            iface: true,
        };
        let ncs = NetCandidateScanner::new()
            .set_accept(acc)
            .set_interface_mode(InterfaceMode::TreatAsIp);
        let res = ncs.find_all(
            b"  ipv4.address1: \"10.20.30.123/24,10.20.30.1\"",
            "(stdin)",
        );
        assert_eq!(
            res,
            vec![
                NetCandidate {
                    range: (18, 33),
                    net: Net::from_str_unchecked("10.20.30.123"),
                },
                NetCandidate {
                    range: (34, 44),
                    net: Net::from_str_unchecked("10.20.30.1"),
                },
            ]
        );
    }

    #[test]
    fn test_interface_mode_treat_as_network() {
        let acc = AcceptSet {
            ip: true,
            net: true,
            oldnet: false,
            iface: true,
        };
        let ncs = NetCandidateScanner::new()
            .set_accept(acc)
            .set_interface_mode(InterfaceMode::TreatAsNetwork);
        let res = ncs.find_all(
            b"  ipv4.address1: \"10.20.30.123/24,10.20.30.1\"",
            "(stdin)",
        );
        assert_eq!(
            res,
            vec![
                NetCandidate {
                    range: (18, 33),
                    net: Net::from_str_unchecked("10.20.30.0/24"),
                },
                NetCandidate {
                    range: (34, 44),
                    net: Net::from_str_unchecked("10.20.30.1"),
                },
            ]
        );
    }

    #[test]
    fn test_interface_mode_complain_and_skip() {
        let acc = AcceptSet {
            ip: true,
            net: true,
            oldnet: false,
            iface: true,
        };
        let ncs = NetCandidateScanner::new()
            .set_accept(acc)
            .set_interface_mode(InterfaceMode::ComplainAndSkip);
        let res = ncs.find_all(
            b"  ipv4.address1: \"10.20.30.123/24,10.20.30.1\"",
            "(stdin)",
        );
        // NOTE: A complaint is logged to stderr (and eaten by the
        // test framework).
        assert_eq!(
            res,
            vec![NetCandidate {
                range: (34, 44),
                net: Net::from_str_unchecked("10.20.30.1"),
            }]
        );
    }

    #[test]
    fn test_accept_only_ip() {
        let acc = AcceptSet {
            ip: true,
            net: false,
            oldnet: false,
            iface: false,
        };
        let ncs = NetCandidateScanner::new().set_accept(acc);
        let res = ncs.find_all(b"x-11.22.0.0/16-x-12.34.56.78/24-x", "(stdin)");
        assert_eq!(
            res,
            vec![
                NetCandidate {
                    range: (2, 14),
                    net: Net::from_str_unchecked("11.22.0.0"),
                },
                NetCandidate {
                    range: (17, 31),
                    net: Net::from_str_unchecked("12.34.56.78"),
                },
            ]
        );
    }

    #[test]
    fn test_accept_only_net() {
        let acc = AcceptSet {
            ip: false,
            net: true,
            oldnet: false,
            iface: false,
        };
        let ncs = NetCandidateScanner::new().set_accept(acc);

        // 11.22.0.0 is not a net
        // 12.34.56.78/24 is an interface
        let res = ncs.find_all(b"x-11.22.0.0-x-12.34.56.78/24-x", "(stdin)");
        assert_eq!(res, vec![]);

        let res = ncs.find_all(b"x-0.0.0.0/0-x-12.34.0.0/24-x", "(stdin)");
        assert_eq!(
            res,
            vec![
                NetCandidate {
                    range: (2, 11),
                    net: Net::from_str_unchecked("0.0.0.0/0"),
                },
                NetCandidate {
                    range: (14, 26),
                    net: Net::from_str_unchecked("12.34.0.0/24"),
                },
            ]
        );
    }

    #[test]
    fn test_accept_only_oldnet_without_iface() {
        let acc = AcceptSet {
            ip: false,
            net: false,
            oldnet: true,
            iface: false, // relevant for net or oldnet
        };
        let ncs = NetCandidateScanner::new().set_accept(acc);

        let res = ncs.find_all(
            b"0.0.0.0/0     # no, a cidr \
              12.34.56.0/24 # no, a cidr \
              72.62.52.1    # no, an ip \
              3.3.3.3/8     # no, an iface \
              4.4.0.0/255.255.0.0  # yes, oldnet net \
              3.3.3.3/255.255.0.0  # no, oldnet iface \
              123.45.123.45 # no, another ip for good measure",
            "(stdin)",
        );
        assert_eq!(
            res,
            vec![NetCandidate {
                range: (109, 128),
                net: Net::from_str_unchecked("4.4.0.0/16"),
            },]
        );
    }

    #[test]
    fn test_accept_only_oldnet_with_iface() {
        let acc = AcceptSet {
            ip: false,
            net: false,
            oldnet: true,
            iface: true, // relevant for net or oldnet
        };
        let ncs = NetCandidateScanner::new()
            .set_accept(acc)
            .set_interface_mode(InterfaceMode::TreatAsNetwork);

        let res = ncs.find_all(
            b"0.0.0.0/0     # no, a cidr \
              12.34.56.0/24 # no, a cidr \
              72.62.52.1    # no, an ip \
              3.3.3.3/8     # no, an iface \
              4.4.0.0/255.255.0.0  # yes, oldnet net \
              3.3.3.3/255.255.0.0  # yes, oldnet iface \
              123.45.123.45 # no, another ip for good measure",
            "(stdin)",
        );
        assert_eq!(
            res,
            vec![
                NetCandidate {
                    range: (109, 128),
                    net: Net::from_str_unchecked("4.4.0.0/16"),
                },
                NetCandidate {
                    range: (148, 167),
                    net: Net::from_str_unchecked("3.3.0.0/16"),
                },
            ]
        );
    }
}
