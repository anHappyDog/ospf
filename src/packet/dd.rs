use crate::lsa::LinkStateAdvertisementHeader;

use super::{OspfPacket, OspfPacketHeader};

pub const FEATURE_BIT_I: u8 = 1 << 2;
pub const FEATURE_BIT_M: u8 = 1 << 1;
pub const FEATURE_BIT_MS: u8 = 1 << 0;

pub struct DataDescriptionPacket {
    pub header: OspfPacketHeader,
    pub interface_mtu: u16,
    pub options: u8,
    pub features: u8,
    pub dd_sequence_number: u32,
    pub lsa_headers: Vec<LinkStateAdvertisementHeader>,
}

impl OspfPacket for DataDescriptionPacket {}

impl DataDescriptionPacket {}
