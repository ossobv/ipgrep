use crate::net::{Net, NetError};

#[derive(Debug)]
pub struct Needle {
    pub src: String,
    pub net: Net,
    pub is_negated: bool,
}

/// Known IP aliases and their corresponding network representations.
/// https://datatracker.ietf.org/doc/html/rfc6890
const IP_ALIASES: &[(&[&str], &[&str])] = &[
    // "all"
    (&["all", "any", "ip"], &["ip4", "ip6"]),
    (&["ip4", "ipv4", "v4"], &["0.0.0.0/0"]),
    (&["ip6", "ipv6", "v6"], &["::/0"]),
    // "global"
    (&["glo", "global"], &["global4", "global6"]),
    (
        &["glo4", "global4"],
        &[
            "ip4",
            "!benchmark4",
            "!doc4",
            "!linklocal4",
            "!lo4",
            "!multicast4",
            "!reserved4",
            "!rfc1918",
            "!shared4",
            "!zeronet",
        ],
    ),
    (
        &["glo6", "global6"],
        &["ip6", "!doc6", "!linklocal6", "!lo6", "!multicast6"],
    ),
    // "private"
    (&["priv", "private"], &["private4", "private6"]),
    (
        &["priv4", "private4"],
        &["linklocal4", "loopback4", "rfc1918", "shared4"],
    ),
    (&["priv6", "private6"], &["fc00::/7"]),
    // "linklocal"
    (&["linklocal"], &["linklocal4", "linklocal6"]),
    (&["linklocal4"], &["169.254.0.0/16"]),
    (&["linklocal6"], &["fe80::/10"]),
    // "loopback"
    (
        &["lo", "localhost", "loopback"],
        &["localhost4", "localhost6"],
    ),
    (&["lo4", "localhost4", "loopback4"], &["127.0.0.0/8"]),
    (&["lo6", "localhost6", "loopback6"], &["::1/128"]),
    // "multicast"
    (&["multicast"], &["multicast4", "multicast6"]),
    (&["multicast4"], &["224.0.0.0/4"]),
    (&["multicast6"], &["ff00::/8"]),
    // "benchmark", "reserved4", "rfc1918", "shared4", "zeronet"
    (&["benchmark4"], &["198.18.0.0/15"]),
    (&["reserved4"], &["240.0.0.0/4"]),
    (
        &["rfc1918"],
        &["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"],
    ),
    (&["shared4"], &["100.64.0.0/10"]), // a.k.a. CGNAT
    (&["zeronet"], &["0.0.0.0/8"]),
    // "documentation"
    (
        &["doc", "documentation"],
        &["documentation4", "documentation6"],
    ),
    (
        &["doc4", "documentation4"],
        &["192.0.2.0/24", "198.51.100.0/24", "203.0.113.0/24"],
    ),
    (&["doc6", "documentation6"], &["2001:db8::/32"]),
];

impl Needle {
    /// Parses a string into one or more Needles.
    /// Recursively handles 1-to-N aliases.
    pub fn parse(s: &str) -> Result<Vec<Self>, NetError> {
        let (input, is_negated) = if let Some(rest) = s.strip_prefix('!') {
            (rest, true)
        } else {
            (s, false)
        };

        // Attempt to find the input in our aliases map.
        let resolved_nets = IP_ALIASES
            .iter()
            .find(|(aliases, _)| {
                aliases.iter().any(|&a| input.eq_ignore_ascii_case(a))
            })
            .map(|(_, nets)| *nets);

        let mut needles = Vec::new();

        if let Some(nets) = resolved_nets {
            // In IP_ALIASES, recurse to find the next class or IP/network.
            for &net_str in nets {
                let mut child_needles = Self::parse(net_str)?;
                for needle in &mut child_needles {
                    needle.is_negated ^= is_negated;
                }
                needles.extend(child_needles);
            }
        } else {
            // Not in IP_ALIASES, parse IP/network.
            let mut needle = Self::try_from(input)?;
            needle.is_negated ^= is_negated;
            needles.push(needle);
        }

        Ok(needles)
    }

    pub fn try_from(s: &str) -> Result<Self, NetError> {
        let (input, is_negated) = if let Some(rest) = s.strip_prefix('!') {
            (rest, true)
        } else {
            (s, false)
        };

        let net = Net::try_from(input)?;

        // Reject if host bits are set.
        if net.has_host_bits() {
            return Err(NetError::HostBitsSet(input.to_string()));
        }

        Ok(Needle {
            src: input.to_string(),
            net,
            is_negated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_classes() {
        for (keys, _) in IP_ALIASES {
            let ns = Needle::parse(keys[0]).unwrap();
            // If it's 0 it will behave as "any", so that would not be good.
            assert!(ns.len() > 0);
        }
    }

    #[test]
    fn test_class_global() {
        let ns = Needle::parse("global").unwrap();
        assert_eq!(ns.len(), 19);
        // ip4
        assert_eq!(ns[0].net, Net::from_str_unchecked("0.0.0.0/0"));
        assert!(!ns[0].is_negated);
        // benchmark4
        assert_eq!(ns[1].net, Net::from_str_unchecked("198.18.0.0/15"));
        assert!(ns[1].is_negated);
        // doc4
        assert_eq!(ns[2].net, Net::from_str_unchecked("192.0.2.0/24"));
        assert_eq!(ns[3].net, Net::from_str_unchecked("198.51.100.0/24"));
        assert_eq!(ns[4].net, Net::from_str_unchecked("203.0.113.0/24"));
        // linklocal4
        assert_eq!(ns[5].net, Net::from_str_unchecked("169.254.0.0/16"));
        // lo4
        assert_eq!(ns[6].net, Net::from_str_unchecked("127.0.0.0/8"));
        // multicast4
        assert_eq!(ns[7].net, Net::from_str_unchecked("224.0.0.0/4"));
        // reserved4
        assert_eq!(ns[8].net, Net::from_str_unchecked("240.0.0.0/4"));
        // rfc1918
        assert_eq!(ns[9].net, Net::from_str_unchecked("10.0.0.0/8"));
        assert_eq!(ns[10].net, Net::from_str_unchecked("172.16.0.0/12"));
        assert_eq!(ns[11].net, Net::from_str_unchecked("192.168.0.0/16"));
        // shared4
        assert_eq!(ns[12].net, Net::from_str_unchecked("100.64.0.0/10"));
        // zeronet
        assert_eq!(ns[13].net, Net::from_str_unchecked("0.0.0.0/8"));
        // ip6
        assert_eq!(ns[14].net, Net::from_str_unchecked("::/0"));
        assert!(!ns[14].is_negated);
        // doc6
        assert_eq!(ns[15].net, Net::from_str_unchecked("2001:db8::/32"));
        assert!(ns[15].is_negated);
        // linklocal6
        assert_eq!(ns[16].net, Net::from_str_unchecked("fe80::/10"));
        // lo6
        assert_eq!(ns[17].net, Net::from_str_unchecked("::1/128"));
        // multicast6
        assert_eq!(ns[18].net, Net::from_str_unchecked("ff00::/8"));
    }

    #[test]
    fn test_needle_class_ip() {
        let ns = Needle::parse("ip").unwrap();
        assert_eq!(ns.len(), 2);
        assert_eq!(ns[0].net, Net::from_str_unchecked("0.0.0.0/0"));
        assert!(!ns[0].is_negated);
        assert_eq!(ns[1].net, Net::from_str_unchecked("::/0"));
        assert!(!ns[1].is_negated);
    }

    #[test]
    fn test_needle_class_negated_multicast() {
        let ns = Needle::parse("!multicast").unwrap();
        assert_eq!(ns.len(), 2);
        assert_eq!(ns[0].net, Net::from_str_unchecked("224.0.0.0/4"));
        assert!(ns[0].is_negated);
        assert_eq!(ns[1].net, Net::from_str_unchecked("ff00::/8"));
        assert!(ns[1].is_negated);
    }

    #[test]
    fn test_needle_valid_ip() {
        let n = Needle::try_from("123.123.123.123").unwrap();
        assert_eq!(n.src, "123.123.123.123");
        assert_eq!(n.net, Net::from_str_unchecked("123.123.123.123/32"));
        assert!(!n.is_negated);
    }

    #[test]
    fn test_needle_valid_negated_ip() {
        let n = Needle::try_from("!123.123.123.123").unwrap();
        assert_eq!(n.src, "123.123.123.123");
        assert_eq!(n.net, Net::from_str_unchecked("123.123.123.123/32"));
        assert!(n.is_negated);
    }

    #[test]
    fn test_needle_valid_net() {
        let n = Needle::try_from("88.99.128.0/17").unwrap();
        assert_eq!(n.src, "88.99.128.0/17");
        assert_eq!(n.net, Net::from_str_unchecked("88.99.128.0/17"));
        assert!(!n.is_negated);
    }

    #[test]
    fn test_needle_valid_negated_net() {
        let n = Needle::try_from("!88.99.128.0/17").unwrap();
        assert_eq!(n.src, "88.99.128.0/17");
        assert_eq!(n.net, Net::from_str_unchecked("88.99.128.0/17"));
        assert!(n.is_negated);
    }

    #[test]
    fn test_needle_valid_oldnet() {
        let n = Needle::try_from("192.168.32.0/255.255.224.0").unwrap();
        assert_eq!(n.src, "192.168.32.0/255.255.224.0");
        assert_eq!(n.net, Net::from_str_unchecked("192.168.32.0/19"));
    }
}
