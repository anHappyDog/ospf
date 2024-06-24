use core::net;
use std::fmt::Debug;

use pnet::packet::ipv4::Ipv4Packet;

use crate::{ALL_DROTHERS, ALL_SPF_ROUTERS};

pub mod dd;
pub mod hello;
pub mod lsack;
pub mod lsr;
pub mod lsu;

pub const IPV4_PACKET_MTU: usize = 1500;

#[derive(Clone, Copy)]
pub struct OspfHeader {
    pub version: u8,
    pub packet_type: u8,
    pub packet_length: u16,
    pub router_id: u32,
    pub area_id: u32,
    pub checksum: u16,
    pub auth_type: u16,
    pub authentication: [u8; 8],
}

impl Debug for OspfHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OspfHeader")
            .field("version", &self.version)
            .field("packet_type", &self.packet_type)
            .field("packet_length", &self.packet_length)
            .field("router_id", &self.router_id)
            .field("area_id", &self.area_id)
            .field("checksum", &self.checksum)
            .field("auth_type", &self.auth_type)
            .field("authentication", &self.authentication)
            .finish()
    }
}

pub enum OspfPacket {
    Hello(hello::Hello),
    DD(dd::DD),
    LSU(lsu::Lsu),
    LSR(lsr::Lsr),
    LSACK(lsack::Lsack),
}

impl OspfPacket {}

impl OspfHeader {
    pub fn empty() -> OspfHeader {
        OspfHeader {
            version: 0,
            packet_type: 0,
            packet_length: 0,
            router_id: 0,
            area_id: 0,
            checksum: 0,
            auth_type: 0,
            authentication: [0; 8],
        }
    }
    pub fn try_from_be_bytes(payload: &[u8]) -> Option<Self> {
        if payload.len() < Self::length() {
            return None;
        }
        Some(Self {
            version: payload[0],
            packet_type: payload[1],
            packet_length: u16::from_be_bytes([payload[2], payload[3]]),
            router_id: u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]),
            area_id: u32::from_be_bytes([payload[8], payload[9], payload[10], payload[11]]),
            checksum: u16::from_be_bytes([payload[12], payload[13]]),
            auth_type: u16::from_be_bytes([payload[14], payload[15]]),
            authentication: [
                payload[16],
                payload[17],
                payload[18],
                payload[19],
                payload[20],
                payload[21],
                payload[22],
                payload[23],
            ],
        })
    }
    pub fn to_le_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.version);
        bytes.push(self.packet_type);
        bytes.extend_from_slice(&self.packet_length.to_le_bytes());
        bytes.extend_from_slice(&self.router_id.to_le_bytes());
        bytes.extend_from_slice(&self.area_id.to_le_bytes());
        bytes.extend_from_slice(&self.checksum.to_le_bytes());
        bytes.extend_from_slice(&self.auth_type.to_le_bytes());
        bytes.extend_from_slice(&self.authentication);
        bytes
    }
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.version);
        bytes.push(self.packet_type);
        bytes.extend_from_slice(&self.packet_length.to_be_bytes());
        bytes.extend_from_slice(&self.router_id.to_be_bytes());
        bytes.extend_from_slice(&self.area_id.to_be_bytes());
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.auth_type.to_be_bytes());
        bytes.extend_from_slice(&self.authentication);
        bytes
    }
    pub fn length() -> usize {
        24
    }
}

/// # ospf_packet_checksum
/// this function will calcuate and return the checksum of the ospf packet
/// the packet is passed by the form of an u8 slice.
pub fn ospf_packet_checksum(packet: &[u8]) -> u16 {
    let mut sum = 0u32;
    for i in 0..packet.len() {
        if i == 12 || i == 13 {
            continue;
        }
        if i % 2 == 0 {
            sum += u32::from(packet[i]) << 8;
        } else {
            sum += u32::from(packet[i]);
        }
    }
    while sum > 0xffff {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

pub fn is_ipv4_packet_valid_for_ospf<'a>(
    ipv4_packet: &'a Ipv4Packet,
    host_addr: net::Ipv4Addr,
) -> bool {
    if ipv4_packet.get_source() == host_addr {
        return false;
    }
    let destination = ipv4_packet.get_destination();
    if destination != ALL_DROTHERS && destination != host_addr && destination != ALL_SPF_ROUTERS {
        return false;
    }
    true
}
