use std::{net, sync::Arc};

use pnet::packet::{
    ip::IpNextHeaderProtocol,
    ipv4::{Ipv4Packet, MutableIpv4Packet},
};
use tokio::sync::RwLock;

use crate::{
    area,
    interface::{self, NetworkType},
    lsa, neighbor, util, OSPF_IP_PROTOCOL, OSPF_VERSION,
};

use super::ospf_packet_checksum;

pub const HELLO_TYPE: u8 = 1;

#[derive(Clone)]
pub struct Hello {
    pub header: super::OspfHeader,
    pub network_mask: u32,
    pub hello_interval: u16,
    pub options: u8,
    pub router_priority: u8,
    pub router_dead_interval: u32,
    pub designated_router: u32,
    pub backup_designated_router: u32,
    pub neighbors: Vec<u32>,
}

impl Hello {
    /// create and fill a new hello packet.
    pub async fn new(iaddr: net::Ipv4Addr) -> Self {
        let mut packet = Self::empty();
        let interfaces_map = interface::INTERFACE_MAP.read().await;
        let interface = interfaces_map.get(&iaddr).unwrap();
        packet.header.router_id = crate::ROUTER_ID.clone().into();
        packet.header.area_id = interface.area_id;
        packet.network_mask = interface.mask.into();
        packet.hello_interval = interface.hello_interval;
        packet.options = interface.options;
        packet.router_priority = interface.router_priority;
        packet.router_dead_interval = interface.router_dead_interval;
        packet.designated_router = {
            let area_drs = crate::area::DR_MAP.read().await;
            let dr = area_drs.get(&interface.area_id).unwrap().clone();
            dr.into()
        };
        packet.backup_designated_router = {
            let area_bdrs = crate::area::BDR_MAP.read().await;
            let bdr = area_bdrs.get(&interface.area_id).unwrap().clone();
            bdr.into()
        };
        let int_neighbors = neighbor::INT_NEIGHBORS_MAP.read().await;
        let int_neighbors = int_neighbors.get(&iaddr).unwrap();
        let int_neighbors = int_neighbors.read().await;
        int_neighbors.iter().for_each(|neighbor| {
            packet.neighbors.push(*neighbor);
        });
        packet.header.packet_length = packet.length() as u16;
        packet.header.checksum = ospf_packet_checksum(&packet.to_be_bytes());
        packet
    }

    /// remember the process for the neighbors in the packet may be not processed properly.
    pub async fn received(
        hello_packet: Hello,
        packet_source_addr: net::Ipv4Addr,
        int_ipv4_addr: net::Ipv4Addr,
    ) {

    }
    pub fn try_from_be_bytes(payload: &[u8]) -> Option<Self> {
        let ospf_header = match super::OspfHeader::try_from_be_bytes(payload) {
            Some(ospf_header) => ospf_header,
            None => return None,
        };
        let mut neighbors = Vec::new();
        for i in 40..payload.len() {
            neighbors.push(u32::from_be_bytes([
                payload[i],
                payload[i + 1],
                payload[i + 2],
                payload[i + 3],
            ]));
        }
        Some(Self {
            header: ospf_header,
            network_mask: u32::from_be_bytes([payload[24], payload[25], payload[26], payload[27]]),
            hello_interval: u16::from_be_bytes([payload[28], payload[29]]),
            options: payload[30],
            router_priority: payload[31],
            router_dead_interval: u32::from_be_bytes([
                payload[32],
                payload[33],
                payload[34],
                payload[35],
            ]),
            designated_router: u32::from_be_bytes([
                payload[36],
                payload[37],
                payload[38],
                payload[39],
            ]),
            backup_designated_router: u32::from_be_bytes([
                payload[40],
                payload[41],
                payload[42],
                payload[43],
            ]),
            neighbors,
        })
    }
    pub fn empty() -> Hello {
        Hello {
            header: super::OspfHeader::empty(),
            network_mask: 0,
            hello_interval: 0,
            options: 0,
            router_priority: 0,
            router_dead_interval: 0,
            designated_router: 0,
            backup_designated_router: 0,
            neighbors: Vec::new(),
        }
    }
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.to_be_bytes());
        bytes.extend_from_slice(&self.network_mask.to_be_bytes());
        bytes.extend_from_slice(&self.hello_interval.to_be_bytes());
        bytes.push(self.options);
        bytes.push(self.router_priority);
        bytes.extend_from_slice(&self.router_dead_interval.to_be_bytes());
        bytes.extend_from_slice(&self.designated_router.to_be_bytes());
        bytes.extend_from_slice(&self.backup_designated_router.to_be_bytes());
        for neighbor in &self.neighbors {
            bytes.extend_from_slice(&neighbor.to_be_bytes());
        }
        bytes
    }
    pub fn length(&self) -> usize {
        44 + 4 * self.neighbors.len()
    }
    pub async fn build_ipv4_packet<'a>(
        &'a self,
        buffer: &'a mut Vec<u8>,
        src_ipv4_addr: net::Ipv4Addr,
    ) -> Result<Ipv4Packet<'a>, &'static str> {
        let network_type = {
            let interfaces_map = interface::INTERFACE_MAP.read().await;
            let interface = interfaces_map.get(&src_ipv4_addr).unwrap();
            interface.network_type
        };
        let mut packet = match MutableIpv4Packet::new(buffer.as_mut_slice()) {
            Some(packet) => packet,
            None => return Err("Failed to create MutableIpv4Packet"),
        };
        packet.set_header_length(5);
        packet.set_version(4);
        packet.set_total_length((self.length() + 20) as u16);
        packet.set_ttl(1);
        packet.set_next_level_protocol(IpNextHeaderProtocol::new(OSPF_IP_PROTOCOL));
        packet.set_source(src_ipv4_addr);
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
}

impl std::fmt::Debug for Hello {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hello")
            .field("header", &self.header)
            .field("network_mask", &self.network_mask)
            .field("hello_interval", &self.hello_interval)
            .field("options", &self.options)
            .field("router_priority", &self.router_priority)
            .field("router_dead_interval", &self.router_dead_interval)
            .field("designated_router", &self.designated_router)
            .field("backup_designated_router", &self.backup_designated_router)
            .field("neighbors", &self.neighbors)
            .finish()
    }
}
