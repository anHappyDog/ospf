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
        length += self.header.length();
        length += self.lsrs.len() * LinkStatusRequest::length();
        length
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
}

impl LinkStateRequestPacket {
    pub fn new(header: OspfPacketHeader, lsrs: Vec<Box<LinkStatusRequest>>) -> Self {
        Self { header, lsrs }
    }
}
