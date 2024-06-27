
#[derive(Clone)]
pub struct NetworkLSA {
    pub header: super::Header,
    pub mask: u32,
    pub attached_rtrs: Vec<u32>,
}

impl NetworkLSA {
    pub fn length(&self) -> usize{
        super::Header::length() + 4 + self.attached_rtrs.len() * 4 
    }
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.to_be_bytes());
        bytes.extend_from_slice(&self.mask.to_be_bytes());
        for rtr in &self.attached_rtrs {
            bytes.extend_from_slice(&rtr.to_be_bytes());
        }
        bytes
    }
}

pub const NETWORK_LSA_TYPE: u8 = 2;


