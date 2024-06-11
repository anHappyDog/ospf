use super::{OspfPacket, OspfPacketHeader};
use crate::lsa::LinkStateAdvertisement;


pub struct LinkStateUpdatePacket {
    pub header: OspfPacketHeader,
    pub lsa_count: u32,
    pub lsas: Vec<Box<dyn LinkStateAdvertisement>>,
}

impl OspfPacket for LinkStateUpdatePacket {}

impl LinkStateUpdatePacket {}
