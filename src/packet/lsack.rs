use crate::lsa::LinkStateAdvertisementHeader;

use super::{OspfPacket, OspfPacketHeader};

pub struct LinkStateAcknowledgementPacket {
    pub header: OspfPacketHeader,
    pub lsa_headers: Vec<Box<LinkStateAdvertisementHeader>>,
}

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
        length += self.header.length();
        for lsa_header in &self.lsa_headers {
            length += lsa_header.length();
        }
        length
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
}
