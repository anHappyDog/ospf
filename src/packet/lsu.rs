use super::{OspfPacket, OspfPacketHeader};
use crate::lsa::{get_lsa_from_be_bytes, LinkStateAdvertisement};

pub struct LinkStateUpdatePacket {
    pub header: OspfPacketHeader,
    pub lsa_count: u32,
    pub lsas: Vec<Box<dyn LinkStateAdvertisement>>,
}

pub const LINK_STATE_UPDATE_TYPE: u8 = 4;

unsafe impl Send for LinkStateUpdatePacket {}

impl OspfPacket for LinkStateUpdatePacket {
    fn to_be_bytes(&self) -> Vec<u8> {
        let mut result = vec![];
        result.extend(self.header.to_be_bytes());
        result.extend(self.lsa_count.to_be_bytes());
        for lsa in &self.lsas {
            result.extend(lsa.to_be_bytes());
        }
        result
    }

    fn calculate_checksum(&mut self) {}
    fn length(&self) -> usize {
        let mut length = 0;
        length += OspfPacketHeader::length();
        length += 4;
        for lsa in &self.lsas {
            length += lsa.length();
        }
        length
    }
    fn get_type(&self) -> u8 {
        LINK_STATE_UPDATE_TYPE
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
    pub fn from_be_bytes(data: &[u8]) -> Self {
        let header = OspfPacketHeader::from_be_bytes(&data[0..24]);
        let lsa_count = u32::from_be_bytes([data[24], data[25], data[26], data[27]]);
        let mut lsas = vec![];
        let mut index = 28;
        while index < data.len() {
            let lsa = get_lsa_from_be_bytes(&data[index..]);
            index += lsa.length() as usize;
            lsas.push(lsa);
        }
        Self {
            header,
            lsa_count,
            lsas,
        }
    }
}
