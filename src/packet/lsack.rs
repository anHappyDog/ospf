use crate::lsa::LinkStateAdvertisementHeader;

use super::{OspfPacket, OspfPacketHeader};

pub struct LinkStateAcknowledgementPacket {
    pub header: OspfPacketHeader,
    pub lsa_headers: Vec<Box<LinkStateAdvertisementHeader>>,
}

pub const LINK_STATE_ACKNOWLEDGEMENT_PACKET_TYPE: u8 = 5;

unsafe impl Send for LinkStateAcknowledgementPacket {}

impl OspfPacket for LinkStateAcknowledgementPacket {
    fn to_bytes(&self) -> Vec<u8> {
        let mut result = vec![];
        result.extend(self.header.to_bytes());
        for lsa_header in &self.lsa_headers {
            result.extend(lsa_header.to_bytes());
        }
        result
    }
    fn length(&self) -> usize {
        let mut length = 0;
        length += OspfPacketHeader::length();
        length += self.lsa_headers.len() * LinkStateAdvertisementHeader::length();
        length
    }
    fn get_type(&self) -> u8 {
        LINK_STATE_ACKNOWLEDGEMENT_PACKET_TYPE
    }
    fn calculate_checksum(&mut self) {}
    fn to_be_bytes(&self) -> Vec<u8> {
        let mut result = vec![];
        result.extend(self.header.to_be_bytes());
        for lsa_header in &self.lsa_headers {
            result.extend(lsa_header.to_be_bytes());
        }
        result
    }
}

impl LinkStateAcknowledgementPacket {
    pub fn new(
        header: OspfPacketHeader,
        lsa_headers: Vec<Box<LinkStateAdvertisementHeader>>,
    ) -> Self {
        Self {
            header,
            lsa_headers: lsa_headers,
        }
    }
    pub fn from_be_bytes(data : &[u8]) -> Self {
        let header = OspfPacketHeader::from_be_bytes(&data[0..24]);
        let mut lsa_headers = vec![];
        let mut index = 24;
        while index < data.len() {
            lsa_headers.push(Box::new(LinkStateAdvertisementHeader::from_be_bytes(&data[index..index+20])));
            index += 20;
        }
        Self {
            header,
            lsa_headers: lsa_headers,
        }
    }
}
