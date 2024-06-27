use std::{net, sync::Arc};

use pnet::packet::{
    ip::IpNextHeaderProtocol,
    ipv4::{Ipv4Packet, MutableIpv4Packet},
};
use tokio::sync::RwLock;

use crate::{
    interface::{self, NetworkType, INTERFACES},
    lsa, neighbor, util, OSPF_IP_PROTOCOL, OSPF_VERSION,
};

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
    /// remember the process for the neighbors in the packet may be not processed properly.
    pub async fn received(
        hello_packet: Hello,
        packet_source_addr: net::Ipv4Addr,
        int_ipv4_addr: net::Ipv4Addr,
    ) {
        let interfaces_map = INTERFACES.read().await;
        let interface = match interfaces_map.get(&int_ipv4_addr) {
            Some(interface) => interface,
            None => {
                util::error("Interface not found.");
                return ();
            }
        };
        let locked_interface = interface.read().await;
        let hello_interval = locked_interface.hello_interval;
        let area_id = locked_interface.area_id;
        let int_ipv4_addr = locked_interface.ip;
        let network_type = locked_interface.network_type;
        let options = locked_interface.options;
        let router_dead_interval = locked_interface.router_dead_interval;
        drop(locked_interface);
        drop(interfaces_map);
        if hello_packet.header.version != OSPF_VERSION || hello_packet.header.area_id != area_id {
            util::error("Invalid ospf version or in-compatible area id.");
            return;
        }
        // currently omit more detailed checks.
        if hello_packet.hello_interval != hello_interval {
            util::error("Invalid hello interval for the received hello ospf packet..");
            return;
        }
        if hello_packet.options != options {
            util::error("Invalid options for the received hello ospf packet.");
            return;
        }
        if hello_packet.router_dead_interval != router_dead_interval {
            util::error("Invalid router dead interval for the received hello ospf packet.");
            return;
        }
        // now we get the right ospf hello packet.
        let (source_addr, neighbor_id) = match network_type {
            NetworkType::Broadcast | NetworkType::NBMA | NetworkType::PointToMultipoint => {
                (packet_source_addr, hello_packet.header.router_id.into())
            }
            NetworkType::PointToPoint | NetworkType::VirtualLink => (
                net::Ipv4Addr::from(hello_packet.header.router_id),
                packet_source_addr,
            ),
        };
        let neighbors = neighbor::NEIGHBORS.write().await;
        let int_neighbors = match neighbors.get(&int_ipv4_addr) {
            Some(int_neighbors) => int_neighbors,
            None => {
                util::error("Interface neighbors not found.");
                return;
            }
        };
        let mut locked_int_neighbors = int_neighbors.write().await;
        let mut locked_neighbor_inactive_timer = neighbor::handle::INACTIVE_TIMERS.write().await;
        locked_neighbor_inactive_timer.insert(source_addr, None);
        drop(locked_neighbor_inactive_timer);

        let former_priority = if !locked_int_neighbors.contains_key(&source_addr) {
            let neighbor = Arc::new(RwLock::new(neighbor::Neighbor {
                state: neighbor::status::Status::Down,
                master: false,
                dd_seq: lsa::INITIAL_SEQUENCE_NUMBER as u32,
                last_dd: None,
                id: neighbor_id,
                priority: hello_packet.router_priority.into(),
                ipv4_addr: source_addr,
                options: hello_packet.options,
                dr: hello_packet.designated_router.into(),
                bdr: hello_packet.backup_designated_router.into(),
                lsa_retrans_list: Vec::new(),
                summary_list: Vec::new(),
                lsr_list: Vec::new(),
            }));

            locked_int_neighbors.insert(source_addr, neighbor.clone());
            neighbor::event::add_sender(source_addr).await;
            let mut sm = neighbor::handle::STATUS_MACHINES.write().await;
            sm.insert(
                source_addr,
                Some(tokio::spawn(neighbor::status::changed(
                    network_type,
                    source_addr,
                    int_ipv4_addr,
                ))),
            );
            hello_packet.router_priority
        } else {
            let neighbor = match locked_int_neighbors.get(&source_addr) {
                Some(neighbor) => neighbor,
                None => {
                    util::error("Neighbor not found.");
                    return;
                }
            };
            let locked_neighbor = neighbor.read().await;
            locked_neighbor.priority as u8
        };
        drop(locked_int_neighbors);
        drop(neighbors);
        neighbor::event::send(source_addr, neighbor::event::Event::HelloReceived).await;
        if hello_packet.neighbors.contains(&int_ipv4_addr.into()) {
            neighbor::event::send(source_addr, neighbor::event::Event::TwoWayReceived).await;
            if former_priority != hello_packet.router_priority {
                interface::event::send(int_ipv4_addr, interface::event::Event::NeighborChange)
                    .await;
            }
            let ints = interface::INTERFACES.read().await;
            let int = match ints.get(&int_ipv4_addr) {
                Some(int) => int,
                None => {
                    util::error("Interface not found.");
                    return;
                }
            };
            let locked_int = int.read().await;
            let int_status = locked_int.status;
            drop(locked_int);
            drop(ints);
            if int_status == interface::status::Status::Waiting
                && hello_packet.designated_router == source_addr.into()
                && u32::from(hello_packet.backup_designated_router) == 0
            {
                interface::event::send(int_ipv4_addr, interface::event::Event::BackupSeen).await;
            } else {
                let locked_neighbors = neighbor::NEIGHBORS.read().await;
                let int_neighbors = match locked_neighbors.get(&int_ipv4_addr) {
                    Some(int_neighbors) => int_neighbors,
                    None => {
                        util::error("Interface neighbors not found.");
                        return;
                    }
                };
                let locked_int_neighbors = int_neighbors.read().await;
                let int_neighbor = match locked_int_neighbors.get(&source_addr) {
                    Some(int_neighbor) => int_neighbor,
                    None => {
                        util::error("Neighbor not found.");
                        return;
                    }
                };
                let locked_int_neighbor = int_neighbor.read().await;
                let former_dr = locked_int_neighbor.dr;
                drop(locked_int_neighbor);
                drop(locked_int_neighbors);
                drop(locked_neighbors);
                if hello_packet.designated_router != former_dr.into()
                    && (former_dr == source_addr || former_dr == source_addr)
                {
                    interface::event::send(int_ipv4_addr, interface::event::Event::NeighborChange)
                        .await;
                }
            }

            //

            if hello_packet.backup_designated_router == source_addr.into()
                && int_status == interface::status::Status::Waiting
            {
                interface::event::send(int_ipv4_addr, interface::event::Event::BackupSeen).await;
            } else {
                let locked_neighbors = neighbor::NEIGHBORS.read().await;
                let int_neighbors = match locked_neighbors.get(&int_ipv4_addr) {
                    Some(int_neighbors) => int_neighbors,
                    None => {
                        util::error("Interface neighbors not found.");
                        return;
                    }
                };
                let locked_int_neighbors = int_neighbors.read().await;
                let int_neighbor = match locked_int_neighbors.get(&source_addr) {
                    Some(int_neighbor) => int_neighbor,
                    None => {
                        util::error("Neighbor not found.");
                        return;
                    }
                };
                let locked_int_neighbor = int_neighbor.read().await;
                let former_bdr = locked_int_neighbor.bdr;
                drop(locked_int_neighbor);
                drop(locked_int_neighbors);
                drop(locked_neighbors);
                if hello_packet.backup_designated_router != former_bdr.into()
                    && (former_bdr == source_addr || former_bdr == source_addr)
                {
                    interface::event::send(int_ipv4_addr, interface::event::Event::NeighborChange)
                        .await;
                }
            }
        } else {
            neighbor::event::send(source_addr, neighbor::event::Event::OneWayReceived).await;
        }
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
    pub fn build_ipv4_packet<'a>(
        &'a self,
        buffer: &'a mut Vec<u8>,
        src_ipv4_addr: net::Ipv4Addr,
        network_type: interface::NetworkType,
    ) -> Result<Ipv4Packet<'a>, &'static str> {
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
