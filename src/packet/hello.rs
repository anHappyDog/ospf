use pnet::packet::ipv4::Ipv4Packet;

use crate::neighbor::Neighbor;
use crate::{interface, neighbor, OSPF_VERSION_2};

use super::{OspfPacket, OspfPacketHeader};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
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
    pub neighbors: Arc<Mutex<HashMap<net::Ipv4Addr, neighbor::Neighbor>>>,
}

pub const HELLO_PACKET_TYPE: u8 = 1;

unsafe impl Send for HelloPacket {}

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
        let neighbors = self.neighbors.lock().unwrap();
        for (neighbor_ip, _) in neighbors.iter() {
            bytes.extend_from_slice(&neighbor_ip.octets());
        }
        bytes
    }

    fn calculate_checksum(&mut self) {
        self.header.checksum = 0;
    }

    fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.length());
        bytes.extend_from_slice(&self.header.to_be_bytes());
        bytes.extend_from_slice(&self.network_mask.octets());
        bytes.extend_from_slice(&self.hello_interval.to_be_bytes());
        bytes.push(self.options);
        bytes.push(self.rtr_pri);
        bytes.extend_from_slice(&self.router_dead_interval.to_be_bytes());
        bytes.extend_from_slice(&self.designated_router.to_be_bytes());
        bytes.extend_from_slice(&self.backup_designated_router.to_be_bytes());
        let neighbors = self.neighbors.lock().unwrap();
        for (neighbor_ip, _) in neighbors.iter() {
            bytes.extend_from_slice(&neighbor_ip.octets());
        }
        bytes
    }
    fn get_type(&self) -> u8 {
        HELLO_PACKET_TYPE
    }
    fn ipv4packet(&self) -> Result<Ipv4Packet, &'static str> {
        Err("not an ipv4 packet")
    }
    fn length(&self) -> usize {
        let mut length = 0;
        length += mem::size_of::<OspfPacketHeader>();
        length += mem::size_of::<net::Ipv4Addr>();
        length += mem::size_of::<u16>();
        length += mem::size_of::<u8>() * 2;
        length += mem::size_of::<u32>() * 3;
        length += mem::size_of::<net::Ipv4Addr>() * self.neighbors.lock().unwrap().len();
        length
    }
}

impl HelloPacket {
    pub fn set_auth_type(&mut self, auth_type: u8) {
        self.header.auth_type = auth_type;
    }
    pub fn set_auth_key(&mut self, auth_key: u64) {
        self.header.authentication = auth_key.to_be_bytes();
    }

    pub fn new_from_interface(
        ip_addr: net::Ipv4Addr,
        area_id: net::Ipv4Addr,
        network_mask: net::Ipv4Addr,
        hello_interval: u16,
    ) -> Self {
        let global_neighbors = crate::NEIGHBORS.clone();
        let locked_neighbors = global_neighbors.lock().unwrap();
        let neighbors = locked_neighbors
            .get(&ip_addr)
            .expect("get neighbors from new hello packet failed.");
        let router_id = crate::ROUTER_ID.clone();

        let header = OspfPacketHeader::new(
            OSPF_VERSION_2,
            HELLO_PACKET_TYPE,
            OspfPacketHeader::length() as u16,
            router_id.into(),
            area_id.into(),
            0,
            0,
            0,
        );
        HelloPacket {
            header,
            network_mask,
            hello_interval: hello_interval,
            options: 0,
            rtr_pri: 0,
            router_dead_interval: 0,
            designated_router: 0,
            backup_designated_router: 0,
            neighbors: neighbors.clone(),
        }
    }

    pub fn new(
        network_mask: net::Ipv4Addr,
        hello_interval: u16,
        options: u8,
        rtr_pri: u8,
        router_dead_interval: u32,
        designated_router: u32,
        backup_designated_router: u32,
        header: OspfPacketHeader,
        neighbors: Arc<Mutex<HashMap<net::Ipv4Addr, neighbor::Neighbor>>>,
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
    pub fn from_be_bytes(
        bytes: &[u8],
        neighbors: Arc<Mutex<HashMap<net::Ipv4Addr, neighbor::Neighbor>>>,
    ) -> Self {
        let header = OspfPacketHeader::from_be_bytes(&bytes[0..24]);
        let network_mask = net::Ipv4Addr::new(bytes[24], bytes[25], bytes[26], bytes[27]);
        let hello_interval = u16::from_be_bytes([bytes[28], bytes[29]]);
        let options = bytes[30];
        let rtr_pri = bytes[31];
        let router_dead_interval = u32::from_be_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]);
        let designated_router = u32::from_be_bytes([bytes[36], bytes[37], bytes[38], bytes[39]]);
        let backup_designated_router =
            u32::from_be_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]);
        let mut locked_neighbors = neighbors.lock().unwrap();
        // for i in 44..bytes.len() {
        //     let neighbor_ip: net::Ipv4Addr =
        //         net::Ipv4Addr::new(bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]);
            
        //     let neighbor: neighbor::Neighbor = neighbor::Neighbor{
        //         ipv4_addr: neighbor_ip,
        //         status: neighbor::status::NeighborStatus::Down,
        //         dead_timer: None,
        //         neighbor_id: 0,
        //         neighbor_priority: 0,
        //         neighbor_designated_router: 0,
        //         neighbor_backup_designated_router: 0,
        //     };
        //     locked_neighbors.insert(neighbor_ip, neighbor);
        // }
        drop(locked_neighbors);
        Self {
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
