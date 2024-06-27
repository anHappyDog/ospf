use std::net;

use pnet::packet::{
    ip::IpNextHeaderProtocol,
    ipv4::{Ipv4Packet, MutableIpv4Packet},
};

use crate::{interface, OSPF_IP_PROTOCOL};

pub const LSU_TYPE: u8 = 4;

pub struct Lsu {
    pub header : super::OspfHeader,
    pub lsa_count: u32,
    pub lsa_list: Vec<crate::lsa::Lsa>,
}

impl Lsu {
    pub fn try_from_be_bytes(payload: &[u8]) -> Option<Self> {
        unimplemented!()
    }
    pub fn length(&self) -> usize {
        super::OspfHeader::length() + 4 + self.lsa_list.iter().fold(0, |acc, lsa| acc + lsa.length())
    }
    pub async fn received(lsu_packet: Lsu) {
        unimplemented!()
    }
    pub fn empty() -> Self {
        Self {
            header: super::OspfHeader::empty(),
            lsa_count: 0,
            lsa_list: Vec::new(),
        }
    }
    pub fn build_ipv4_packet<'a>(
        &'a self,
        buffer: &'a mut Vec<u8>,
        network_type: interface::NetworkType,
        int_ipv4_addr: net::Ipv4Addr,
        destination_addr: net::Ipv4Addr,
    ) -> Result<Ipv4Packet<'a>, &'static str> {
        let mut packet = match MutableIpv4Packet::new(buffer.as_mut_slice()) {
            Some(packet) => packet,
            None => return Err("Failed to create MutableIpv4Packet"),
        };
        packet.set_header_length(5);
        packet.set_version(4);
        packet.set_total_length((self.length() + 20) as u16);
        packet.set_ttl(1);
        packet.set_next_level_protocol(IpNextHeaderProtocol::new(OSPF_IP_PROTOCOL));
        packet.set_source(int_ipv4_addr);
        match network_type {
            interface::NetworkType::Broadcast => {
                packet.set_destination([224, 0, 0, 5].into());
            }
            interface::NetworkType::PointToPoint => {
                packet.set_destination([224, 0, 0, 5].into());
            }
            interface::NetworkType::NBMA => {
                packet.set_destination([224, 0, 0, 5].into());
            }
            interface::NetworkType::PointToMultipoint => {
                packet.set_destination([224, 0, 0, 5].into());
            }
            interface::NetworkType::VirtualLink => {
                packet.set_destination([224, 0, 0, 5].into());
            }
        }
        packet.set_payload(&self.to_be_bytes());
        packet.set_checksum(pnet::packet::ipv4::checksum(&packet.to_immutable()));
        Ok(packet.consume_to_immutable())
    }
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.to_be_bytes());
        bytes.extend_from_slice(&self.lsa_count.to_be_bytes());
        self.lsa_list.iter().for_each(|lsa| {
            bytes.extend_from_slice(&lsa.to_be_bytes());
        });
        bytes
    }
}
