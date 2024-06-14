pub mod dd;
pub mod hello;
pub mod lsack;
pub mod lsr;
pub mod lsu;

use hello::HELLO_PACKET_TYPE;
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::{self, Ipv4Packet, MutableIpv4Packet};
use pnet::packet::{MutablePacket, Packet};
use pnet::transport::{transport_channel, TransportChannelType::Layer3};
use pnet::{transport, util};
use std::net;

use crate::{debug, OSPF_IP_PROTOCOL_NUMBER, OSPF_VERSION_2};

pub struct OspfPacketHeader {
    pub version: u8,
    pub packet_type: u8,
    pub packet_length: u16,
    pub router_id: u32,
    pub area_id: u32,
    pub checksum: u16,
    pub auth_type: u8,
    pub authentication: [u8; 8],
}

pub fn calculate_checksum(data: &[u16]) -> u16 {
    let mut sum: u32 = 0;
    for &word in data {
        sum += word as u32;
    }

    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !(sum as u16)
}

impl OspfPacketHeader {
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(24);
        bytes.push(self.version);
        bytes.push(self.packet_type);
        bytes.extend_from_slice(&self.packet_length.to_be_bytes());
        bytes.extend_from_slice(&self.router_id.to_be_bytes());
        bytes.extend_from_slice(&self.area_id.to_be_bytes());
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.push(self.auth_type);
        bytes.extend_from_slice(&self.authentication);
        bytes
    }
    pub fn from_be_bytes(bytes: &[u8]) -> Self {
        Self {
            version: bytes[0],
            packet_type: bytes[1],
            packet_length: u16::from_be_bytes([bytes[2], bytes[3]]),
            router_id: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            area_id: u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            checksum: u16::from_be_bytes([bytes[12], bytes[13]]),
            auth_type: bytes[14],
            authentication: [
                bytes[15], bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21],
                bytes[22],
            ],
        }
    }
    pub fn new(
        version: u8,
        packet_type: u8,
        packet_length: u16,
        router_id: u32,
        area_id: u32,
        checksum: u16,
        auth_type: u8,
        auth_key: u64,
    ) -> Self {
        Self {
            version,
            packet_type,
            packet_length,
            router_id,
            area_id,
            checksum,
            auth_type,
            authentication: auth_key.to_be_bytes(),
        }
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(24);
        bytes.push(self.version);
        bytes.push(self.packet_type);
        bytes.extend_from_slice(&self.packet_length.to_be_bytes());
        bytes.extend_from_slice(&self.router_id.to_be_bytes());
        bytes.extend_from_slice(&self.area_id.to_be_bytes());
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.push(self.auth_type);
        bytes.extend_from_slice(&self.authentication);
        bytes
    }
    pub fn length() -> usize {
        24
    }
}

// please remember that in ospf the packket must not and does not have to be sliced.
pub(self) fn new_ip_packets(
    buffer: &mut [u8],
    src_ip: net::Ipv4Addr,
    dst_ip: net::Ipv4Addr,
    packet: Vec<u8>,
) -> Result<Box<MutableIpv4Packet>, &'static str> {
    if packet.len() >= 1500 - 20 {
        return Err("packet too big");
    }
    let mut ip_packet = Box::new(MutableIpv4Packet::new(buffer).expect("create ip packet failed"));
    ip_packet.set_version(4);
    ip_packet.set_header_length(5);
    ip_packet.set_total_length(20 + packet.len() as u16);
    ip_packet.set_ttl(1);
    ip_packet.set_next_level_protocol(IpNextHeaderProtocols::Udp);
    ip_packet.set_source(src_ip);
    ip_packet.set_destination(dst_ip);
    ip_packet.set_payload(&packet);
    Ok(ip_packet)
}

pub fn try_get_from_ipv4_packet<'a>(
    ip_packet: &Ipv4Packet,
    hello_neighbors: &'a mut Vec<net::Ipv4Addr>,
) -> Result<Box<dyn OspfPacket + 'a>, &'static str> {
    if ip_packet.get_dscp() != OSPF_IP_PROTOCOL_NUMBER {
        return Err("not an ospf packet");
    }
    let ospf_packet = ip_packet.payload();
    if ospf_packet.len() < OspfPacketHeader::length() {
        return Err("packet too small");
    }
    if ospf_packet[0] != OSPF_VERSION_2 {
        return Err("not an ospf version 2 packet");
    }
    let ospf_packet: Box<dyn OspfPacket> = match ospf_packet[1] {
        crate::packet::hello::HELLO_PACKET_TYPE => Box::new(hello::HelloPacket::from_be_bytes(
            ospf_packet,
            hello_neighbors,
        )),
        crate::packet::dd::DATA_DESCRIPTION_PACKET_TYPE => {
            Box::new(dd::DataDescriptionPacket::from_be_bytes(ospf_packet))
        }
        crate::packet::lsr::LINK_STATE_REQUEST_PACKET_TYPE => {
            Box::new(lsr::LinkStateRequestPacket::from_be_bytes(ospf_packet))
        }
        crate::packet::lsack::LINK_STATE_ACKNOWLEDGEMENT_PACKET_TYPE => Box::new(
            lsack::LinkStateAcknowledgementPacket::from_be_bytes(ospf_packet),
        ),
        crate::packet::lsu::LINK_STATE_UPDATE_TYPE => {
            Box::new(lsu::LinkStateUpdatePacket::from_be_bytes(ospf_packet))
        }
        _ => return Err("unknown packet type"),
    };
    Ok(ospf_packet)
}

pub trait OspfPacket {
    fn length(&self) -> usize;
    fn to_bytes(&self) -> Vec<u8>;
    fn to_be_bytes(&self) -> Vec<u8>;
    fn calculate_checksum(&mut self);
    fn get_type(&self) -> u8;
}

pub fn send_to(
    ospf_packet: &dyn OspfPacket,
    tx: &mut transport::TransportSender,
    src_ip: net::Ipv4Addr,
    dst_ip: net::Ipv4Addr,
) -> Result<(), &'static str> {
    let mut buffer = vec![0; ospf_packet.length() + 20];
    let ip_packet = new_ip_packets(&mut buffer, src_ip, dst_ip, ospf_packet.to_be_bytes())
        .expect("create ip packet failed");
    tx.send_to(ip_packet, net::IpAddr::V4(dst_ip))
        .expect("send packet failed");
    Ok(())
}
