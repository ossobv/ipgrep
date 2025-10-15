use memchr::memchr2_iter;

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
        Self { accept, ..self }
    }

    pub fn set_interface_mode(self, interface_mode: InterfaceMode) -> Self {
        Self {
            interface_mode,
            ..self
        }
    }

    pub fn find_all(&self, buf: &[u8]) -> Vec<NetCandidate> {
        let mut candidates = Vec::new();

        // This actually produces quite a speedup.
        if !prefilter_could_be_ip(buf) {
            return candidates;
        }

        // FIXME: maybe pass !(accept.ip || accept.net || accept.iface)
        // here too, so we can skip all "/"-matching in that case.
        let netlikescanner = if self.accept.oldnet {
            NetLikeScanner::new(buf).with_oldnet(true)
        } else {
            NetLikeScanner::new(buf)
        };

        for (start, end) in netlikescanner {
            let slice = &buf[start..end];

            // Do we only want networks?
            if !self.accept.ip && !slice.contains(&b'/') {
                continue;
            }

            // FIXME: do something with self.accept.oldnet here:
            // IpNet::with_netmask(ip, mask)

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
                        // FIXME: complain and treat at IP? or..
                        eprintln!("WARN: Ignoring host-bit-set net {net}");
                        continue;
                    }
                }
            }

            // If we found an IP, check that we're doing Needle scans on those.
            if !self.include_ipv6 && net.is_ipv6() {
                // FIXME: handle IPv4-in-IPv6 selection/cases? "::ffff:1.2.3.4"
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
        let res =
            ncs.find_all(b"  ipv4.address1: \"10.20.30.123/24,10.20.30.1\"");
        assert_eq!(
            res,
            vec![
                NetCandidate {
                    range: (18, 33),
                    net: Net::try_from("10.20.30.123").expect("no fail"),
                },
                NetCandidate {
                    range: (34, 44),
                    net: Net::try_from("10.20.30.1").expect("no fail"),
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
        let res =
            ncs.find_all(b"  ipv4.address1: \"10.20.30.123/24,10.20.30.1\"");
        assert_eq!(
            res,
            vec![
                NetCandidate {
                    range: (18, 33),
                    net: Net::try_from("10.20.30.0/24").expect("no fail"),
                },
                NetCandidate {
                    range: (34, 44),
                    net: Net::try_from("10.20.30.1").expect("no fail"),
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
        let res =
            ncs.find_all(b"  ipv4.address1: \"10.20.30.123/24,10.20.30.1\"");
        // FIXME: where is the complaint logged?
        assert_eq!(
            res,
            vec![NetCandidate {
                range: (34, 44),
                net: Net::try_from("10.20.30.1").expect("no fail"),
            }]
        );
    }
}
