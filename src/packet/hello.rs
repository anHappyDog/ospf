use crate::neighbor;

use super::{OspfPacket, OspfPacketHeader};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::{Ipv4Packet, MutableIpv4Packet};
use pnet::packet::{Packet, PacketSize};
use pnet::transport::{transport_channel, TransportChannelType::Layer3};
use pnet::{transport, util};
use std::{mem, net};
/// # struct HelloPacket
/// - header : the ospf packet header
/// doc to be implemented
pub struct HelloPacket {
    pub header: OspfPacketHeader,
    pub network_mask: net::Ipv4Addr,
    pub hello_interval: u16,
    pub options: u8,
    pub rtr_pri: u8,
    pub router_dead_interval: u32,
    pub designated_router: u32,
    pub backup_designated_router: u32,
    pub neighbors: Vec<net::Ipv4Addr>,
}

impl OspfPacket for HelloPacket {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.length());
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.network_mask.octets());
        bytes.extend_from_slice(&self.hello_interval.to_be_bytes());
        bytes.push(self.options);
        bytes.push(self.rtr_pri);
        bytes.extend_from_slice(&self.router_dead_interval.to_be_bytes());
        bytes.extend_from_slice(&self.designated_router.to_be_bytes());
        bytes.extend_from_slice(&self.backup_designated_router.to_be_bytes());
        for neighbor in &self.neighbors {
            bytes.extend_from_slice(&neighbor.octets());
        }
        bytes
    }

    fn length(&self) -> usize {
        let mut length = 0;
        length += mem::size_of::<OspfPacketHeader>();
        length += mem::size_of::<net::Ipv4Addr>();
        length += mem::size_of::<u16>();
        length += mem::size_of::<u8>() * 2;
        length += mem::size_of::<u32>() * 3;
        length += mem::size_of::<net::Ipv4Addr>() * self.neighbors.len();
        length
    }
}

impl HelloPacket {
    pub fn new(
        network_mask: net::Ipv4Addr,
        hello_interval: u16,
        options: u8,
        rtr_pri: u8,
        router_dead_interval: u32,
        designated_router: u32,
        backup_designated_router: u32,
        header: OspfPacketHeader,
        neighbors: Vec<net::Ipv4Addr>,
    ) -> Self {
        HelloPacket {
            header,
            network_mask,
            hello_interval,
            options,
            rtr_pri,
            router_dead_interval,
            designated_router,
            backup_designated_router,
            neighbors,
        }
    }
}
