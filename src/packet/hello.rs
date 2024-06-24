use std::net;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::{ip::IpNextHeaderProtocol, ipv4::MutableIpv4Packet};
use std::fmt::Debug;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::interface::{NetworkType, INTERFACES_BY_NAME};
use crate::{err, interface, neighbor, util, OSPF_IP_PROTOCOL, OSPF_VERSION};

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

impl Debug for Hello {
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

    pub fn to_le_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.to_le_bytes());
        bytes.extend_from_slice(&self.network_mask.to_le_bytes());
        bytes.extend_from_slice(&self.hello_interval.to_le_bytes());
        bytes.push(self.options);
        bytes.push(self.router_priority);
        bytes.extend_from_slice(&self.router_dead_interval.to_le_bytes());
        bytes.extend_from_slice(&self.designated_router.to_le_bytes());
        bytes.extend_from_slice(&self.backup_designated_router.to_le_bytes());
        for neighbor in &self.neighbors {
            bytes.extend_from_slice(&neighbor.to_le_bytes());
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
        util::debug(&format!(
            "Building Hello packet,pass buffer len is {}，self.length is {}",
            buffer.len(),
            self.length()
        ));
        let mut packet = match MutableIpv4Packet::new(buffer.as_mut_slice()) {
            Some(packet) => packet,
            None => return Err("Failed to create MutableIpv4Packet"),
        };
        util::debug(&format!(
            "Building Hello packet,pass MutableIpv4Packet,packet header length is {}",
            packet.get_header_length() * 4,
        ));
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
        println!("Building Hello packet,pass set_destination, ip length is {},head length is {},self length is {}",packet.get_total_length(),packet.get_header_length(),self.length());
        packet.set_payload(&self.to_be_bytes());

        packet.set_checksum(pnet::packet::ipv4::checksum(&packet.to_immutable()));
        util::debug("Building Hello packet,pass set_checksum");
        Ok(packet.consume_to_immutable())
    }
}

pub async fn when_received(
    packet_source_addr: Ipv4Addr,
    hello_packet: Hello,
    interface_name: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let interfaces_by_name_map = INTERFACES_BY_NAME.read().await;
    let interface = match interfaces_by_name_map.get(&interface_name) {
        Some(interface) => interface,
        None => {
            util::error("Interface not found.");
            return Err(Box::new(err::OspfError::new(
                "Interface not found.".to_string(),
            )));
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
    drop(interfaces_by_name_map);
    if hello_packet.header.version != OSPF_VERSION || hello_packet.header.area_id != area_id {
        util::error("Invalid ospf version or in-compatible area id.");
        return Err(Box::new(err::OspfError::new(
            "Invalid ospf version or in-compatible area id.".to_string(),
        )));
    }
    // currently omit more detailed checks.
    if hello_packet.hello_interval != hello_interval {
        util::error("Invalid hello interval for the received hello ospf packet..");
        return Err(Box::new(err::OspfError::new(
            "Invalid hello interval for the received hello ospf packet.".to_string(),
        )));
    }
    if hello_packet.options != options {
        util::error("Invalid options for the received hello ospf packet.");
        return Err(Box::new(err::OspfError::new(
            "Invalid options for the received hello ospf packet.".to_string(),
        )));
    }
    if hello_packet.router_dead_interval != router_dead_interval {
        util::error("Invalid router dead interval for the received hello ospf packet.");
        return Err(Box::new(err::OspfError::new(
            "Invalid router dead interval for the received hello ospf packet.".to_string(),
        )));
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
            return Err(Box::new(err::OspfError::new(
                "Interface neighbors not found.".to_string(),
            )));
        }
    };
    let mut locked_int_neighbors = int_neighbors.write().await;
    let former_priority = if !locked_int_neighbors.contains_key(&source_addr) {
        let neighbor = Arc::new(RwLock::new(neighbor::Neighbor {
            state: neighbor::status::Status::Down,
            inactive_timer: None,
            master: false,
            dd_seq: 0,
            last_dd: None,
            id: neighbor_id,
            priority: hello_packet.router_priority.into(),
            ipv4_addr: source_addr,
            options: hello_packet.options,
            dr: hello_packet.designated_router.into(),
            bdr: hello_packet.backup_designated_router.into(),
        }));

        locked_int_neighbors.insert(source_addr, neighbor.clone());
        hello_packet.router_priority
    } else {
        let neighbor = match locked_int_neighbors.get(&source_addr) {
            Some(neighbor) => neighbor,
            None => {
                util::error("Neighbor not found.");
                return Err(Box::new(err::OspfError::new(
                    "Neighbor not found.".to_string(),
                )));
            }
        };
        let locked_neighbor = neighbor.read().await;
        locked_neighbor.priority as u8
    };
    drop(locked_int_neighbors);
    drop(neighbors);
    neighbor::status_changed(
        int_ipv4_addr,
        source_addr,
        neighbor::event::Event::HelloReceived,
    )
    .await?;
    if hello_packet.neighbors.contains(&int_ipv4_addr.into()) {
        neighbor::status_changed(
            int_ipv4_addr,
            source_addr,
            neighbor::event::Event::TwoWayReceived,
        )
        .await?;
        if former_priority != hello_packet.router_priority {
            // interface::status::status_changed(
            //     interface_name.clone(),
            //     interface::event::Event::NeighborChange,
            // )
            // .await?;
            tokio::try_join!(interface::status::status_changed(
                interface_name.clone(),
                interface::event::Event::NeighborChange,
            ));
        }
        let ints = interface::INTERFACES_BY_NAME.read().await;
        let int = match ints.get(&interface_name) {
            Some(int) => int,
            None => {
                util::error("Interface not found.");
                return Err(Box::new(err::OspfError::new(
                    "Interface not found.".to_string(),
                )));
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
            interface::status::status_changed(
                interface_name.clone(),
                interface::event::Event::BackupSeen,
            )
            .await?;
        } else {
            let locked_neighbors = neighbor::NEIGHBORS.read().await;
            let int_neighbors = match locked_neighbors.get(&int_ipv4_addr) {
                Some(int_neighbors) => int_neighbors,
                None => {
                    util::error("Interface neighbors not found.");
                    return Err(Box::new(err::OspfError::new(
                        "Interface neighbors not found.".to_string(),
                    )));
                }
            };
            let locked_int_neighbors = int_neighbors.read().await;
            let int_neighbor = match locked_int_neighbors.get(&source_addr) {
                Some(int_neighbor) => int_neighbor,
                None => {
                    util::error("Neighbor not found.");
                    return Err(Box::new(err::OspfError::new(
                        "Neighbor not found.".to_string(),
                    )));
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
                interface::status::status_changed(
                    interface_name.clone(),
                    interface::event::Event::NeighborChange,
                )
                .await?;
            }
        }

        //

        if hello_packet.backup_designated_router == source_addr.into()
            && int_status == interface::status::Status::Waiting
        {
            interface::status::status_changed(
                interface_name.clone(),
                interface::event::Event::BackupSeen,
            )
            .await?;
        } else {
            let locked_neighbors = neighbor::NEIGHBORS.read().await;
            let int_neighbors = match locked_neighbors.get(&int_ipv4_addr) {
                Some(int_neighbors) => int_neighbors,
                None => {
                    util::error("Interface neighbors not found.");
                    return Err(Box::new(err::OspfError::new(
                        "Interface neighbors not found.".to_string(),
                    )));
                }
            };
            let locked_int_neighbors = int_neighbors.read().await;
            let int_neighbor = match locked_int_neighbors.get(&source_addr) {
                Some(int_neighbor) => int_neighbor,
                None => {
                    util::error("Neighbor not found.");
                    return Err(Box::new(err::OspfError::new(
                        "Neighbor not found.".to_string(),
                    )));
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
                interface::status::status_changed(
                    interface_name.clone(),
                    interface::event::Event::NeighborChange,
                )
                .await?;
            }
        }
    } else {
        neighbor::status_changed(
            int_ipv4_addr,
            source_addr,
            neighbor::event::Event::OneWayReceived,
        )
        .await?;
    }
    Ok(())
}
