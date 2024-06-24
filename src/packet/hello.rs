use core::net;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::{ip::IpNextHeaderProtocol, ipv4::MutableIpv4Packet};
use std::sync::Arc;

use crate::{interface, OSPF_IP_PROTOCOL};

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
    pub fn checksum(&self) -> usize {
        0
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
        40 + 4 * self.neighbors.len()
    }
    pub fn build_ipv4_packet<'a>(
        &'a self,
        buffer: &'a mut Vec<u8>,
        src_ipv4_addr: net::Ipv4Addr,
        network_type: interface::NetworkType,
    ) -> Ipv4Packet<'a> {
        let mut packet = MutableIpv4Packet::new(buffer).unwrap();
        packet.set_version(4);
        packet.set_header_length(5);
        packet.set_total_length((self.length() + 5 * 4) as u16);
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
        packet.consume_to_immutable()
    }
}


pub async fn when_received(hello_packet : Hello,interface_name : String) {
    
}

