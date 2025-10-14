use std::convert::TryFrom;
use std::fmt;
use std::net::IpAddr;
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
    pub fn has_host_bits(&self) -> bool {
        self.0.network() != self.0.addr()
    }

    pub fn is_ipv4(&self) -> bool {
        matches!(self.0, IpNet::V4(_ipnet))
    }

    pub fn is_ipv6(&self) -> bool {
        matches!(self.0, IpNet::V6(_ipnet))
    }

    pub fn as_ip(self) -> Self {
        Net(IpNet::new(self.0.addr(), self.0.max_prefix_len())
            .expect("cannot fail"))
    }

    pub fn as_network(self) -> Self {
        Net(IpNet::new(self.0.network(), self.0.prefix_len())
            .expect("cannot fail"))
    }
}

impl TryFrom<&str> for Net {
    type Error = NetError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if let Ok(net) = s.parse::<IpNet>() {
            Ok(Net(net))
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
