use super::{OspfPacket,OspfPacketHeader};
use std::net;

/// # struct HelloPacket
/// - header : the ospf packet header
/// doc to be implemented
pub struct HelloPacket {
    pub header: OspfPacketHeader,
    pub network_mask :  net::Ipv4Addr,
    pub hello_interval : u16,
    pub options : u8,
    pub rtr_pri : u8,
    pub router_dead_interval : u32,
    pub designated_router : u32,
    pub backup_designated_router :u32,
    pub neighbors: Vec<net::Ipv4Addr>,
}



impl OspfPacket for HelloPacket {
}

impl HelloPacket {
    
}