use crate::net::{Net, NetError};

#[derive(Debug)]
pub struct Needle {
    pub src: String,
    pub net: Net,
}

impl Needle {
    pub fn try_from(s: &str) -> Result<Self, NetError> {
        let net = Net::try_from(s)?;

        // Reject if host bits are set.
        if net.has_host_bits() {
            return Err(NetError::HostBitsSet(s.to_string()));
        }

        Ok(Needle {
            src: s.to_string(),
            net,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needle_valid_ip() {
        let n = Needle::try_from("123.123.123.123").unwrap();
        assert_eq!(n.src, "123.123.123.123");
        assert_eq!(n.net, Net::from_str_unchecked("123.123.123.123/32"));
    }

    #[test]
    fn test_needle_valid_net() {
        let n = Needle::try_from("88.99.128.0/17").unwrap();
        assert_eq!(n.src, "88.99.128.0/17");
        assert_eq!(n.net, Net::from_str_unchecked("88.99.128.0/17"));
    }

    #[test]
    fn test_needle_valid_oldnet() {
        let n = Needle::try_from("192.168.32.0/255.255.224.0").unwrap();
        assert_eq!(n.src, "192.168.32.0/255.255.224.0");
        assert_eq!(n.net, Net::from_str_unchecked("192.168.32.0/19"));
    }
}
