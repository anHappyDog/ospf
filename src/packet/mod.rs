pub mod hello;
pub mod dd;
pub mod lsu;
pub mod lsr;
pub mod lsack; 


pub struct OspfPacketHeader {
    pub version: u8,
    pub packet_type: u8,
    pub packet_length: u16,
    pub router_id: u32,
    pub area_id: u32,
    pub checksum: u16,
    pub auth_type: u8,
    pub authentication: [u8; 8],
}

pub trait OspfPacket {}