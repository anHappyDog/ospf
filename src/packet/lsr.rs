use pnet::packet::ipv4::Ipv4Packet;

use super::{OspfPacket, OspfPacketHeader};

pub struct LinkStatusRequest {
    pub ls_type: u32,
    pub link_state_id: u32,
    pub advertising_router: u32,
}

pub struct LinkStateRequestPacket {
    pub header: OspfPacketHeader,
    pub lsrs: Vec<Box<LinkStatusRequest>>,
}

pub const LINK_STATE_REQUEST_PACKET_TYPE: u8 = 3;

unsafe impl Send for LinkStatusRequest {}
unsafe impl Send for LinkStateRequestPacket {}

impl LinkStatusRequest {
    pub fn new(ls_type: u32, link_state_id: u32, advertising_router: u32) -> Self {
        Self {
            ls_type,
            link_state_id,
            advertising_router,
        }
    }
    pub fn length() -> usize {
        12
    }
}

impl OspfPacket for LinkStateRequestPacket {
    fn length(&self) -> usize {
        let mut length = 0;
        length += OspfPacketHeader::length();
        length += self.lsrs.len() * LinkStatusRequest::length();
        length
    }
    fn ipv4packet(&self) -> Result<Ipv4Packet, &'static str> {
        Err("not an ipv4 packet")
    }
    fn to_bytes(&self) -> Vec<u8> {
        let mut result = vec![];
        result.extend(self.header.to_bytes());
        for lsr in &self.lsrs {
            result.extend(lsr.ls_type.to_be_bytes());
            result.extend(lsr.link_state_id.to_be_bytes());
            result.extend(lsr.advertising_router.to_be_bytes());
        }
        result
    }
    fn calculate_checksum(&mut self) {}
    fn to_be_bytes(&self) -> Vec<u8> {
        let mut result = vec![];
        result.extend(self.header.to_be_bytes());
        for lsr in &self.lsrs {
            result.extend(lsr.ls_type.to_be_bytes());
            result.extend(lsr.link_state_id.to_be_bytes());
            result.extend(lsr.advertising_router.to_be_bytes());
        }
        result
    }
    fn get_type(&self) -> u8 {
        LINK_STATE_REQUEST_PACKET_TYPE
    }
}

impl LinkStateRequestPacket {
    pub fn new(header: OspfPacketHeader, lsrs: Vec<Box<LinkStatusRequest>>) -> Self {
        Self { header, lsrs }
    }
    pub fn from_be_bytes(data: &[u8]) -> Self {
        let header = OspfPacketHeader::from_be_bytes(&data[0..24]);
        let mut lsrs = vec![];
        let mut index = 24;
        while index < data.len() {
            lsrs.push(Box::new(LinkStatusRequest {
                ls_type: u32::from_be_bytes([
                    data[index],
                    data[index + 1],
                    data[index + 2],
                    data[index + 3],
                ]),
                link_state_id: u32::from_be_bytes([
                    data[index + 4],
                    data[index + 5],
                    data[index + 6],
                    data[index + 7],
                ]),
                advertising_router: u32::from_be_bytes([
                    data[index + 8],
                    data[index + 9],
                    data[index + 10],
                    data[index + 11],
                ]),
            }));
            index += 12;
        }
        Self { header, lsrs }
    }
}
