pub mod dd;
pub mod hello;
pub mod lsack;
pub mod lsr;
pub mod lsu;

use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::MutableIpv4Packet;
use pnet::packet::{MutablePacket, Packet};
use pnet::transport::{transport_channel, TransportChannelType::Layer3};
use pnet::{transport, util};
use std::net;

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

impl OspfPacketHeader {
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
    pub fn length(&self) -> usize {
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
    ip_packet.set_next_level_protocol(IpNextHeaderProtocols::Tcp);
    ip_packet.set_source(src_ip);
    ip_packet.set_destination(dst_ip);
    ip_packet.set_payload(&packet);
    Ok(ip_packet)
}

pub trait OspfPacket {
    fn length(&self) -> usize;
    fn to_bytes(&self) -> Vec<u8>;
}

pub fn send_to(
    ospf_packet: &dyn OspfPacket,
    tx: &mut transport::TransportSender,
    src_ip: net::Ipv4Addr,
    dst_ip: net::Ipv4Addr,
) -> Result<(), &'static str> {
    let mut buffer = vec![0; ospf_packet.length() + 20];
    let ip_packet = new_ip_packets(&mut buffer, src_ip, dst_ip, ospf_packet.to_bytes())
        .expect("create ip packet failed");
    tx.send_to(ip_packet, net::IpAddr::V4(dst_ip))
        .expect("send packet failed");
    Ok(())
}
