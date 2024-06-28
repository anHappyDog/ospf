use std::{net, sync::Arc};

use pnet::packet::{
    ip::IpNextHeaderProtocol,
    ipv4::{Ipv4Packet, MutableIpv4Packet},
};
use tokio::sync::RwLock;

use crate::{
    area::lsdb,
    interface::{self, handle::start_send_lsack, NetworkType},
    lsa::{self, Lsa},
    neighbor, OSPF_IP_PROTOCOL, OSPF_VERSION,
};

pub const LSU_TYPE: u8 = 4;

#[derive(Clone)]
pub struct Lsu {
    pub header: super::OspfHeader,
    pub lsa_count: u32,
    pub lsa_list: Vec<crate::lsa::Lsa>,
}

impl Lsu {
    pub fn try_from_be_bytes(payload: &[u8]) -> Option<Self> {
        unimplemented!()
    }
    pub async fn new(
        iaddr: net::Ipv4Addr,
        naddr: net::Ipv4Addr,
        lsas: Vec<Arc<RwLock<Lsa>>>,
    ) -> Self {
        let mut packet = Self::empty();
        let interface = interface::INTERFACE_MAP.read().await;
        let locked_interface = interface.get(&iaddr).unwrap();
        packet.header.version = OSPF_VERSION;
        packet.header.area_id = locked_interface.area_id;
        packet.header.auth_type = locked_interface.auth_type;
        packet.header.authentication = locked_interface.auth_key;
        packet.header.packet_type = LSU_TYPE;
        packet.header.router_id = crate::ROUTER_ID.clone();
        packet.header.packet_length = 0;
        packet.lsa_count = lsas.len() as u32;
        for lsa in lsas.iter() {
            let locked_lsa = lsa.read().await;
            packet.lsa_list.push(locked_lsa.clone());
        }
        packet
    }
    pub fn length(&self) -> usize {
        super::OspfHeader::length()
            + 4
            + self.lsa_list.iter().fold(0, |acc, lsa| acc + lsa.length())
    }
    pub async fn received(lsu_packet: Lsu, naddr: net::Ipv4Addr, iaddr: net::Ipv4Addr) {
        crate::area::lsdb::update_lsdb(iaddr, lsu_packet.lsa_list.clone()).await;
        let lsa_identifiers = lsu_packet.lsa_list.iter().map(|lsa| lsa.copy_header()).collect::<Vec<lsa::Header>>();
        interface::handle::start_send_lsack(iaddr, naddr,Some( lsa_identifiers)).await;
        neighbor::event::send(iaddr, naddr, neighbor::event::Event::LoadingDone).await;
    }
    pub fn empty() -> Self {
        Self {
            header: super::OspfHeader::empty(),
            lsa_count: 0,
            lsa_list: Vec::new(),
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
    pub async fn build_ipv4_packet<'a>(
        lsu : Lsu,
        buffer: &'a mut Vec<u8>,
        iaddr: net::Ipv4Addr,
        naddr: net::Ipv4Addr,
    ) -> Result<Ipv4Packet<'a>, &'static str> {
        let mut packet = match MutableIpv4Packet::new(buffer.as_mut_slice()) {
            Some(packet) => packet,
            None => return Err("Failed to create MutableIpv4Packet"),
        };
        packet.set_header_length(5);
        packet.set_version(4);
        packet.set_total_length((lsu.length() + 20) as u16);
        packet.set_ttl(1);
        packet.set_next_level_protocol(IpNextHeaderProtocol::new(OSPF_IP_PROTOCOL));
        packet.set_source(iaddr);
        let network_type = interface::get_network_type(iaddr).await;
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
                packet.set_destination(naddr);
            }
        }
        packet.set_payload(&lsu.to_be_bytes());
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


