pub mod graph;
pub mod spt;
use std::net;
use std::sync::Arc;

use pnet::packet::{ipv4, Packet};
use tokio::sync::RwLock;

use crate::interface::{self, handle::PACKET_SEND};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DestionationType {
    Network,
    Router,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PathType {
    IntraArea,
    InterArea,
    ExternalType1,
    ExternalType2,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RouteTableEntry {
    pub destination_type: DestionationType,
    pub destination_id: net::Ipv4Addr,
    pub mask: net::Ipv4Addr,
    pub options: u8,
    pub area_id: u32,
    pub path_type: PathType,
    pub cost: u32,
    pub type2_cost: u32,
    pub ls_origin: u32,
    pub next_hop: net::Ipv4Addr,
    pub advertising_router: net::Ipv4Addr,
}

pub struct RouteTable {
    pub entries: Vec<RouteTableEntry>,
}

impl RouteTable {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    pub fn add_entry(&mut self, entry: RouteTableEntry) {
        self.entries.push(entry);
    }
    pub fn remove_entry(&mut self, entry: RouteTableEntry) {
        self.entries.retain(|e| e != &entry);
    }
    pub async fn forward_packet<'a>(&self, ipv4_packet: ipv4::Ipv4Packet<'a>) {
        if ipv4_packet.get_ttl() == 0 {
            crate::util::debug("forwarding:ttl is 0,dropped.");
            return;
        }
        let mut mutable_packet =
            ipv4::MutableIpv4Packet::owned(ipv4_packet.packet().to_vec()).unwrap();
        mutable_packet.set_ttl(mutable_packet.get_ttl() - 1);
        mutable_packet.set_checksum(ipv4::checksum(&mutable_packet.to_immutable()));
        for entry in &self.entries {
            if entry.destination_id == ipv4_packet.get_destination()
                && entry.mask == ipv4_packet.get_destination()
            {
                crate::util::debug("forwarding:found route.");
                let packet_sender = interface::trans::PACKET_SENDER.clone();
                let imm_packet = mutable_packet.consume_to_immutable();
                match packet_sender.send(bytes::Bytes::copy_from_slice(imm_packet.packet())) {
                    Ok(_) => {
                        crate::util::debug("forwarding:send packet success.");
                    }
                    Err(_) => {
                        crate::util::error("forwarding:send packet failed.");
                    }
                }
                return;
            }
        }
        crate::util::error("forwarding:route entry not found.");

    }
}

lazy_static::lazy_static! {
    pub static ref ROUTE_TABLE : Arc<RwLock<RouteTable>> = Arc::new(RwLock::new(RouteTable::new()));
}

pub async fn update_route_table<'a>(ipv4_packet: ipv4::Ipv4Packet<'a>) {
    unimplemented!()
}

pub async fn forward_packet<'a>(iaddr: net::Ipv4Addr, ipv4_packet: ipv4::Ipv4Packet<'a>) {
    let destination = ipv4_packet.get_destination();
    if destination.is_broadcast() || destination.is_multicast() {
        crate::util::debug("forwarding:broadcast or multicast packet,dropped.");
        return;
    }
    if destination.is_loopback() {
        crate::util::debug("forwarding:loopback packet,dropped.");
        return;
    }
    if destination == iaddr {
        crate::util::debug("forwarding: packet to myself,dropped.");
        return;
    } else {
        let route_table = ROUTE_TABLE.read().await;
        route_table.forward_packet(ipv4_packet).await;
    }
}
