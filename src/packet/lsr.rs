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

impl OspfPacket for LinkStateRequestPacket {}

impl LinkStateRequestPacket {}
