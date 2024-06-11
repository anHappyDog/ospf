use crate::lsa::LinkStateAdvertisementHeader;

use super::{OspfPacket, OspfPacketHeader};

pub struct LinkStateAcknowledgementPacket {
    pub header: OspfPacketHeader,
    pub lsa_headers: Vec<Box<LinkStateAdvertisementHeader>>,
}

impl OspfPacket for LinkStateAcknowledgementPacket {}

impl LinkStateAcknowledgementPacket {}
