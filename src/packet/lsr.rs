use core::net;

use pnet::packet::{
    ip::IpNextHeaderProtocol,
    ipv4::{Ipv4Packet, MutableIpv4Packet},
};

use crate::{interface::{self, NetworkType}, OSPF_IP_PROTOCOL};

pub const LSR_TYPE: u8 = 3;
#[derive(Clone)]
pub struct Lsr {
    pub header: super::OspfHeader,
    pub lsr_entries: Vec<LsrEntry>,
}

#[derive(Clone, Copy)]
pub struct LsrEntry {
    pub lsa_type: u32,
    pub lsa_id: u32,
    pub advertising_router: u32,
}

impl LsrEntry {
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.lsa_type.to_be_bytes());
        bytes.extend_from_slice(&self.lsa_id.to_be_bytes());
        bytes.extend_from_slice(&self.advertising_router.to_be_bytes());
        bytes
    }
    pub fn length() -> usize {
        12
    }
    pub fn try_from_be_bytes(payload: &[u8]) -> Option<Self> {
        if payload.len() < 12 {
            return None;
        }
        Some(Self {
            lsa_type: u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]),
            lsa_id: u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]),
            advertising_router: u32::from_be_bytes([payload[8], payload[9], payload[10], payload[11]]),
        })
    }
}

impl Lsr {
    pub fn empty() -> Self {
        Self {
            header: super::OspfHeader::empty(),
            lsr_entries: Vec::new(),
        }
    }
    pub fn get_neighbor_addr(&self, network_type: NetworkType, paddr: net::Ipv4Addr) -> net::Ipv4Addr {
        match network_type {
            NetworkType::Broadcast | NetworkType::NBMA | NetworkType::PointToMultipoint => paddr,
            _ => self.header.router_id,
        }
    }
    pub fn get_neighbor_id(&self, network_type: NetworkType, paddr: net::Ipv4Addr) -> net::Ipv4Addr {
        match network_type {
            NetworkType::Broadcast | NetworkType::NBMA | NetworkType::PointToMultipoint => {
                self.header.router_id
            }
            _ => paddr,
        }
    }
    pub fn try_from_be_bytes(payload: &[u8]) -> Option<Self> {
        if payload.len() < super::OspfHeader::length() {
            return None;
        }
        let header = super::OspfHeader::try_from_be_bytes(&payload[..super::OspfHeader::length()])?;
        let mut lsa_entries = Vec::new();
        let mut offset = super::OspfHeader::length();
        while offset < payload.len() {
            let lsr_entry = LsrEntry::try_from_be_bytes(&payload[offset..])?;
            lsa_entries.push(lsr_entry);
            offset += crate::lsa::Header::length();
        }
        Some(Self {
            header,
            lsr_entries: lsa_entries,
        })
    }
    pub fn length(&self) -> usize {
        super::OspfHeader::length() + LsrEntry::length() * self.lsr_entries.len()
    }
    pub async fn received(lsr_packet: Lsr) {}
    pub fn build_ipv4_packet<'a>(
        &'a self,
        buffer: &'a mut Vec<u8>,
        network_type: interface::NetworkType,
        int_ipv4_addr: net::Ipv4Addr,
        destination_addr: net::Ipv4Addr,
    ) -> Result<Ipv4Packet<'a>, &'static str> {
        let mut packet = match MutableIpv4Packet::new(buffer) {
            Some(packet) => packet,
            None => return Err("Failed to construct packet"),
        };
        packet.set_version(4);
        packet.set_header_length(5);
        packet.set_total_length(20 + self.length() as u16);
        packet.set_ttl(1);
        packet.set_next_level_protocol(IpNextHeaderProtocol(OSPF_IP_PROTOCOL));
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
        for lsa_header in &self.lsr_entries {
            bytes.extend_from_slice(&lsa_header.to_be_bytes());
        }
        bytes
    }
}
