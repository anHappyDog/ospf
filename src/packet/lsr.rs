use core::net;

use pnet::packet::{
    ip::IpNextHeaderProtocol,
    ipv4::{Ipv4Packet, MutableIpv4Packet},
};

use crate::{
    area::{self, lsdb::LsaIdentifer},
    interface::{self, handle::start_send_lsu, NetworkType},
    neighbor, OSPF_IP_PROTOCOL,
};

use super::ospf_packet_checksum;

pub const LSR_TYPE: u8 = 3;
#[derive(Clone)]
pub struct Lsr {
    pub header: super::OspfHeader,
    pub lsr_entries: Vec<LsaIdentifer>,
}

impl Lsr {
    pub async fn new(iaddr: net::Ipv4Addr, lsr_entries: Vec<LsaIdentifer>) -> Self {
        let interface = interface::INTERFACE_MAP.read().await;
        let locked_interface = interface.get(&iaddr).unwrap();
        let mut header = Self::empty();
        header.header.router_id = crate::ROUTER_ID.clone();
        header.header.area_id = locked_interface.area_id;
        header.header.auth_type = locked_interface.auth_type;
        header.header.authentication = locked_interface.auth_key;
        header.header.packet_length = 0;
        header.header.packet_type = LSR_TYPE;
        header.lsr_entries = lsr_entries;
        header.header.packet_length = header.length() as u16;
        header.header.checksum = ospf_packet_checksum(&header.to_be_bytes());
        header
    }
    pub fn empty() -> Self {
        Self {
            header: super::OspfHeader::empty(),
            lsr_entries: Vec::new(),
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
        if payload.len() < super::OspfHeader::length() {
            return None;
        }
        let header = super::OspfHeader::try_from_be_bytes(&payload[..super::OspfHeader::length()])?;
        let mut lsa_entries = Vec::new();
        let mut offset = super::OspfHeader::length();
        while offset < payload.len() {
            let lsr_entry = LsaIdentifer::try_from_be_bytes(&payload[offset..])?;
            lsa_entries.push(lsr_entry);
            offset += crate::lsa::Header::length();
        }
        Some(Self {
            header,
            lsr_entries: lsa_entries,
        })
    }
    pub fn length(&self) -> usize {
        super::OspfHeader::length() + LsaIdentifer::length() * self.lsr_entries.len()
    }
    pub async fn received(lsr_packet: Lsr, naddr: net::Ipv4Addr, iaddr: net::Ipv4Addr) {
        let status = neighbor::get_status(iaddr, naddr).await;
        match status {
            neighbor::status::Status::Exchange
            | neighbor::status::Status::Loading
            | neighbor::status::Status::Full => {
                crate::util::log(&format!("Received LSR packet from {} on {}", naddr, iaddr));
                let lsa_identifiers = lsr_packet.lsr_entries.clone();
                neighbor::fill_retrans_list(iaddr, naddr, lsa_identifiers.clone()).await;
                if let None = area::lsdb::fetch_lsas(iaddr, lsa_identifiers).await {
                    neighbor::event::send(iaddr, naddr, neighbor::event::Event::BadLSReq).await;
                } else {
                    start_send_lsu(iaddr, naddr).await;
                    crate::util::log(&format!(
                        "LSAs sent to {} on {} by lsu packet.",
                        naddr, iaddr
                    ));
                }
            }
            _ => {
                crate::util::error(
                    "Received LSR packet when neighbor's status is not right, discarded.",
                );
                return;
            }
        }
    }
    pub async fn build_ipv4_packet<'a>(
        &'a self,
        buffer: &'a mut Vec<u8>,
        iaddr: net::Ipv4Addr,
        naddr: net::Ipv4Addr,
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
        packet.set_source(iaddr);
        let interface = interface::INTERFACE_MAP.read().await;
        let locked_interface = interface.get(&iaddr).unwrap();
        let network_type = locked_interface.network_type;
        drop(interface);
        match network_type {
            interface::NetworkType::Broadcast => {
                packet.set_destination([224, 0, 0, 5].into());
            }
            interface::NetworkType::PointToPoint => {
                packet.set_destination(naddr);
            }
            interface::NetworkType::NBMA => {
                packet.set_destination([224, 0, 0, 5].into());
            }
            interface::NetworkType::PointToMultipoint => {
                packet.set_destination([224, 0, 0, 5].into());
            }
            interface::NetworkType::VirtualLink => {
                packet.set_destination(naddr);
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
