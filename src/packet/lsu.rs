use super::{OspfPacket, OspfPacketHeader};
use crate::lsa::LinkStateAdvertisement;


pub struct LinkStateUpdatePacket {
    pub header: OspfPacketHeader,
    pub lsa_count: u32,
    pub lsas: Vec<Box<dyn LinkStateAdvertisement>>,
}

impl OspfPacket for LinkStateUpdatePacket {
    fn length(&self) -> usize {
        let mut length = 0;
        length += self.header.length();
        length += 4;
        for lsa in &self.lsas {
            length += lsa.length();
        }
        length
    }
    fn to_bytes(&self) -> Vec<u8> {
        let mut result = vec![];
        result.extend(self.header.to_bytes());
        result.extend(self.lsa_count.to_be_bytes());
        for lsa in &self.lsas {
            result.extend(lsa.to_bytes());
        }
        result
    }
    
}

impl LinkStateUpdatePacket {
    pub fn new(
        header: OspfPacketHeader,
        lsa_count: u32,
        lsas: Vec<Box<dyn LinkStateAdvertisement>>,
    ) -> Self {
        Self {
            header,
            lsa_count,
            lsas,
        }
    }
}
