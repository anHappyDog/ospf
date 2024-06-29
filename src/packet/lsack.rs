use std::{any::Any, net};

use pnet::packet::{
    ip::IpNextHeaderProtocol,
    ipv4::{Ipv4Packet, MutableIpv4Packet},
};

use crate::{
    area::lsdb::LsaIdentifer,
    interface::{self, NetworkType},
    lsa::{self, network},
    neighbor, ALL_SPF_ROUTERS, OSPF_IP_PROTOCOL,
};

use super::ospf_packet_checksum;

pub const LSACK_TYPE: u8 = 5;

pub struct Lsack {
    pub header: super::OspfHeader,
    pub lsa_headers: Vec<crate::lsa::Header>,
}

impl Lsack {
    pub async fn new(
        iaddr: net::Ipv4Addr,
        naddr: net::Ipv4Addr,
        headers: Option<Vec<crate::lsa::Header>>,
    ) -> Self {
        let mut lsack = Self::empty();
        let interfaces = interface::INTERFACE_MAP.read().await;
        let interface = interfaces.get(&iaddr).unwrap();
        lsack.header.router_id = crate::ROUTER_ID.clone();
        lsack.header.area_id = interface.area_id;
        lsack.header.auth_type = interface.auth_type;
        lsack.header.authentication = interface.auth_key;
        lsack.header.packet_length = 0;
        lsack.header.packet_type = LSACK_TYPE;
        if let Some(t) = headers {
            lsack.lsa_headers.extend(t);  
        }
        lsack.header.packet_length = lsack.length() as u16;
        lsack.header.checksum = ospf_packet_checksum(&lsack.to_be_bytes());
        lsack
    }
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.to_be_bytes());
        for lsa_header in &self.lsa_headers {
            bytes.extend_from_slice(&lsa_header.to_be_bytes());
        }
        bytes
    }
    pub fn length(&self) -> usize {
        super::OspfHeader::length() + self.lsa_headers.len() * crate::lsa::Header::length()
    }
    pub fn empty() -> Self {
        Self {
            header: super::OspfHeader::empty(),
            lsa_headers: Vec::new(),
        }
    }
    pub fn get_neighbor_addr(
        &self,
        network_type: NetworkType,
        paddr: net::Ipv4Addr,
    ) -> net::Ipv4Addr {
        match network_type {
            NetworkType::Broadcast | NetworkType::NBMA | NetworkType::PointToMultipoint => paddr,
            _ => self.header.router_id,
        }
    }
    pub fn get_neighbor_id(
        &self,
        network_type: NetworkType,
        paddr: net::Ipv4Addr,
    ) -> net::Ipv4Addr {
        match network_type {
            NetworkType::Broadcast | NetworkType::NBMA | NetworkType::PointToMultipoint => {
                self.header.router_id
            }
            _ => paddr,
        }
    }
    pub fn try_from_be_bytes(payload: &[u8]) -> Option<Self> {
        if payload.len() % crate::lsa::Header::length() != 0 {
            return None;
        }
        let header = super::OspfHeader::try_from_be_bytes(&payload[..super::OspfHeader::length()])?;
        let mut lsa_headers = Vec::new();
        let mut offset = 0;
        while offset < payload.len() {
            let lsa_header = crate::lsa::Header::try_from_be_bytes(&payload[offset..])?;
            lsa_headers.push(lsa_header);
            offset += crate::lsa::Header::length();
        }
        Some(Lsack {
            header,
            lsa_headers,
        })
    }
    pub async fn received(lsack_packet: Lsack, naddr: net::Ipv4Addr, iaddr: net::Ipv4Addr) {
        let old_status = neighbor::get_status(iaddr, naddr).await;
        if old_status < neighbor::status::Status::Exchange {
            crate::util::error("Received LSACK from neighbor in wrong state,discarded.");
            return;
        }
        for lsa in lsack_packet.lsa_headers.iter() {
            let lsa_identifer = LsaIdentifer::from_header(lsa);
            neighbor::ack_lsa(iaddr, naddr, lsa_identifer).await;
        }
    }
    pub async  fn build_ipv4_packet<'a>(
        &'a self,
        buffer: &'a mut Vec<u8>,
        iaddr: net::Ipv4Addr,
        naddr: net::Ipv4Addr,
    ) -> Result<Ipv4Packet<'a>, &'static str> {
        let mut packet = match MutableIpv4Packet::new(buffer) {
            Some(packet) => packet,
            None => return Err("Failed to construct packet"),
        };
        let network_type = interface::get_network_type(iaddr).await;
        packet.set_version(4);
        packet.set_header_length(5);
        packet.set_total_length(20 + self.length() as u16);
        packet.set_ttl(1);
        packet.set_next_level_protocol(IpNextHeaderProtocol(OSPF_IP_PROTOCOL));
        packet.set_source(iaddr);
        match network_type {
            interface::NetworkType::Broadcast => {
                packet.set_destination(ALL_SPF_ROUTERS);
            }
            interface::NetworkType::PointToPoint => {
                packet.set_destination(naddr);
            }
            interface::NetworkType::NBMA => {
                packet.set_destination(ALL_SPF_ROUTERS);
            }
            interface::NetworkType::PointToMultipoint => {
                packet.set_destination(ALL_SPF_ROUTERS);
            }
            interface::NetworkType::VirtualLink => {
                packet.set_destination(naddr);
            }
        }
        packet.set_payload(&self.to_be_bytes());
        packet.set_checksum(pnet::packet::ipv4::checksum(&packet.to_immutable()));
        Ok(packet.consume_to_immutable())
    }
}
