use std::convert::TryFrom;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr};
use std::str::from_utf8;

pub use ipnet::IpNet; // re-export

#[derive(Debug)]
pub enum NetError {
    InvalidUtf8,
    NotAnIp(String),
    HostBitsSet(String),
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetError::InvalidUtf8 => {
                write!(f, "invalid utf-8") // can't happen
            }
            NetError::NotAnIp(s) => {
                write!(f, "invalid ip/net as needle: {s}")
            }
            NetError::HostBitsSet(s) => {
                write!(f, "needle cannot have host bits set: {s}")
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Net(pub IpNet);

impl fmt::Display for Net {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Net {
    pub fn from_str_unchecked(s: &str) -> Self {
        Self::try_from(s)
            .expect("Net::from_str_unchecked: static input must be valid")
    }

    pub fn has_host_bits(&self) -> bool {
        self.0.network() != self.0.addr()
    }

    pub fn is_ipv4(&self) -> bool {
        matches!(self.0, IpNet::V4(_ipnet))
    }

    pub fn is_ipv6(&self) -> bool {
        matches!(self.0, IpNet::V6(_ipnet))
    }

    pub fn as_ip(&self) -> Self {
        Net(IpNet::new(self.0.addr(), self.0.max_prefix_len())
            .expect("cannot fail"))
    }

    pub fn as_network(&self) -> Self {
        Net(IpNet::new(self.0.network(), self.0.prefix_len())
            .expect("cannot fail"))
    }
}

impl TryFrom<&str> for Net {
    type Error = NetError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if let Some((ip_part, mask_part)) = s.split_once('/') {
            if mask_part.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(ipnet) = s.parse::<IpNet>() {
                    return Ok(Net(ipnet));
                }
            } else if let (Ok(ip), Ok(mask)) =
                (ip_part.parse::<Ipv4Addr>(), mask_part.parse::<Ipv4Addr>())
            {
                if let Ok(ipnet) =
                    IpNet::with_netmask(IpAddr::V4(ip), IpAddr::V4(mask))
                {
                    return Ok(Net(ipnet));
                }
            }
            Err(NetError::NotAnIp(s.to_string()))
        } else if let Ok(addr) = s.parse::<IpAddr>() {
            Ok(Net(IpNet::from(addr)))
        } else {
            Err(NetError::NotAnIp(s.to_string()))
        }
    }
}

impl TryFrom<&[u8]> for Net {
    type Error = NetError;

    fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
        let s = from_utf8(s).map_err(|_| NetError::InvalidUtf8)?;
        Net::try_from(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_old_10_0_0_0_255_255_254_0() {
        let new = Net::from_str_unchecked("10.0.0.0/23");
        let old = Net::from_str_unchecked("10.0.0.0/255.255.254.0");
        let old2 = Net::from_str_unchecked("10.0.0.0/255.255.255.0");
        assert_eq!(old, new);
        assert_ne!(old, old2);
    }

    #[test]
    fn test_v4_10_0_0_0_24() {
        let n = Net::from_str_unchecked("10.0.0.0/24");
        assert!(n.is_ipv4());
        assert!(!n.is_ipv6());
        assert!(!n.has_host_bits());
        assert_eq!(n.as_ip(), Net::from_str_unchecked("10.0.0.0/32"));
        assert_eq!(n.as_network(), n);
    }

    #[test]
    fn test_v4_192_168_2_254_24() {
        let n = Net::from_str_unchecked("192.168.2.254/24");
        assert!(n.is_ipv4());
        assert!(!n.is_ipv6());
        assert!(n.has_host_bits());
        assert_eq!(n.as_ip(), Net::from_str_unchecked("192.168.2.254"));
        assert_eq!(n.as_network(), Net::from_str_unchecked("192.168.2.0/24"));
    }

    #[test]
    fn test_v6_fe80_1_64() {
        let n = Net::from_str_unchecked("fe80::1/64");
        assert!(!n.is_ipv4());
        assert!(n.is_ipv6());
        assert!(n.has_host_bits());
        assert_eq!(n.as_ip(), Net::from_str_unchecked("fe80::1"));
        assert_eq!(n.as_network(), Net::from_str_unchecked("fe80::/64"));
    }

    #[test]
    fn test_v6_0_24() {
        let n = Net::from_str_unchecked("::/24");
        assert!(!n.is_ipv4());
        assert!(n.is_ipv6());
        assert!(!n.has_host_bits());
        assert_eq!(n.as_ip(), Net::from_str_unchecked("::/128"));
        assert_eq!(n.as_network(), n);
    }

    #[test]
    fn test_v6_2001_db8_29() {
        let n = Net::from_str_unchecked("2001:db8::/29"); // "/28" has hostbits
        assert!(!n.is_ipv4());
        assert!(n.is_ipv6());
        assert!(!n.has_host_bits());
        assert_eq!(n.as_ip(), Net::from_str_unchecked("2001:db8::"));
        assert_eq!(n.as_network(), n);
    }
}
