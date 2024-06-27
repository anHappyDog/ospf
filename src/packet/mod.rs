use std::net;

use dd::{DD, DD_TYPE};
use hello::{Hello, HELLO_TYPE};
use lsack::{Lsack, LSACK_TYPE};
use lsr::{Lsr, LSR_TYPE};
use lsu::{Lsu, LSU_TYPE};
use pnet::packet::{
    ip,
    ipv4::{self, Ipv4Packet},
    Packet,
};

use crate::{ALL_SPF_ROUTERS, OSPF_IP_PROTOCOL};

pub mod dd;
pub mod hello;
pub mod lsack;
pub mod lsr;
pub mod lsu;

#[derive(Clone, Copy)]
pub struct OspfHeader {
    pub version: u8,
    pub packet_type: u8,
    pub packet_length: u16,
    pub router_id: net::Ipv4Addr,
    pub area_id: net::Ipv4Addr,
    pub checksum: u16,
    pub auth_type: u16,
    pub authentication: u64,
}

impl std::fmt::Debug for OspfHeader {
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

impl OspfHeader {
    pub fn empty() -> OspfHeader {
        OspfHeader {
            version: 0,
            packet_type: 0,
            packet_length: 0,
            router_id: net::Ipv4Addr::new(0, 0, 0, 0),
            area_id: net::Ipv4Addr::new(0, 0, 0, 0),
            checksum: 0,
            auth_type: 0,
            authentication: 0,
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
            router_id: net::Ipv4Addr::new(payload[4], payload[5], payload[6], payload[7]),
            area_id: net::Ipv4Addr::new(payload[8], payload[9], payload[10], payload[11]),
            checksum: u16::from_be_bytes([payload[12], payload[13]]),
            auth_type: u16::from_be_bytes([payload[14], payload[15]]),
            authentication: u64::from_be_bytes([
                payload[16],
                payload[17],
                payload[18],
                payload[19],
                payload[20],
                payload[21],
                payload[22],
                payload[23],
            ]),
        })
    }
    pub fn to_le_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.version);
        bytes.push(self.packet_type);
        bytes.extend_from_slice(&self.packet_length.to_le_bytes());
        let router_id: u32 = self.router_id.into();
        bytes.extend_from_slice(&router_id.to_le_bytes());
        let area_id: u32 = self.area_id.into();
        bytes.extend_from_slice(&area_id.to_le_bytes());
        bytes.extend_from_slice(&self.checksum.to_le_bytes());
        bytes.extend_from_slice(&self.auth_type.to_le_bytes());
        bytes.extend_from_slice(&self.authentication.to_le_bytes());
        bytes
    }
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.version);
        bytes.push(self.packet_type);
        bytes.extend_from_slice(&self.packet_length.to_be_bytes());
        let router_id: u32 = self.router_id.into();
        bytes.extend_from_slice(&router_id.to_be_bytes());
        let area_id: u32 = self.area_id.into();
        bytes.extend_from_slice(&area_id.to_be_bytes());
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.auth_type.to_be_bytes());
        bytes.extend_from_slice(&self.authentication.to_be_bytes());
        bytes
    }
    pub fn length() -> usize {
        24
    }
}

pub enum OspfPacket {
    Hello(hello::Hello),
    DD(dd::DD),
    LSU(lsu::Lsu),
    LSR(lsr::Lsr),
    LSACK(lsack::Lsack),
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
    if destination != crate::ALL_DROTHERS
        && destination != host_addr
        && destination != ALL_SPF_ROUTERS
    {
        return false;
    }
    true
}

impl OspfPacket {
    pub async fn received<'a>(
        ipv4_packet: Ipv4Packet<'a>,
        ospf_packet: OspfPacket,
        ipv4_addr: net::Ipv4Addr,
    ) {
        match ospf_packet {
            OspfPacket::Hello(hello_packet) => {
                tokio::spawn(Hello::received(
                    hello_packet,
                    ipv4_packet.get_source(),
                    ipv4_addr,
                ));
            }
            OspfPacket::DD(dd_packet) => {
                tokio::spawn(DD::received(dd_packet, ipv4_packet.get_source(), ipv4_addr));
            }
            OspfPacket::LSR(lsr_packet) => {
                tokio::spawn(Lsr::received(lsr_packet));
            }
            OspfPacket::LSU(lsu_packet) => {
                tokio::spawn(Lsu::received(lsu_packet));
            }
            OspfPacket::LSACK(lsack_packet) => {
                tokio::spawn(Lsack::received(lsack_packet));
            }
        }
    }
    pub fn try_from_ipv4_packet(
        ipv4_packet: &ipv4::Ipv4Packet,
        interface_name: String,
    ) -> Result<crate::packet::OspfPacket, &'static str> {
        match ipv4_packet.get_next_level_protocol() {
            ip::IpNextHeaderProtocol(OSPF_IP_PROTOCOL) => {
                let payload = ipv4_packet.payload();
                match crate::packet::OspfHeader::try_from_be_bytes(payload) {
                    Some(ospf_header) => match ospf_header.packet_type {
                        HELLO_TYPE => {
                            let hello_packet =
                                crate::packet::hello::Hello::try_from_be_bytes(payload);
                            match hello_packet {
                                Some(hello_packet) => {
                                    crate::util::debug("ospf hello packet received.");
                                    Ok(crate::packet::OspfPacket::Hello(hello_packet))
                                }
                                None => Err("invalid hello packet,ignored."),
                            }
                        }
                        DD_TYPE => {
                            let dd_packet = crate::packet::dd::DD::try_from_be_bytes(payload);
                            match dd_packet {
                                Some(dd_packet) => {
                                    crate::util::debug("ospf dd packet received.");
                                    Ok(crate::packet::OspfPacket::DD(dd_packet))
                                }
                                None => Err("invalid dd packet,ignored."),
                            }
                        }
                        LSR_TYPE => {
                            let lsr_packet = crate::packet::lsr::Lsr::try_from_be_bytes(payload);
                            match lsr_packet {
                                Some(lsr_packet) => {
                                    crate::util::debug("ospf lsr packet received.");
                                    Ok(crate::packet::OspfPacket::LSR(lsr_packet))
                                }
                                None => Err("invalid lsr packet,ignored."),
                            }
                        }
                        LSU_TYPE => {
                            let lsu_packet = crate::packet::lsu::Lsu::try_from_be_bytes(payload);
                            match lsu_packet {
                                Some(lsu_packet) => {
                                    crate::util::debug("ospf lsu packet received.");
                                    Ok(crate::packet::OspfPacket::LSU(lsu_packet))
                                }
                                None => Err("invalid lsu packet,ignored."),
                            }
                        }
                        LSACK_TYPE => {
                            let lsack_packet =
                                crate::packet::lsack::Lsack::try_from_be_bytes(payload);
                            match lsack_packet {
                                Some(lsack_packet) => {
                                    crate::util::debug("ospf lsack packet received.");
                                    Ok(crate::packet::OspfPacket::LSACK(lsack_packet))
                                }
                                None => Err("invalid lsack packet,ignored."),
                            }
                        }
                        _ => Err("invalid ospf packet type,ignored."),
                    },
                    None => Err("invalid ospf packet,ignored."),
                }
            }
            _ => Err("non-ospf packet received."),
        }
    }
}
