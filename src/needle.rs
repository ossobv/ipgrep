use crate::net::{Net, NetError};

#[derive(Debug)]
pub struct Needle {
    pub src: String,
    pub net: Net,
}

impl Needle {
    pub fn try_from(s: &str) -> Result<Self, NetError> {
        let net = Net::try_from(s)?;

        // Reject if host bits are set
        if net.has_host_bits() {
            return Err(NetError::HostBitsSet(s.to_string()));
        }

        Ok(Needle {
            src: s.to_string(),
            net,
        })
    }
}
