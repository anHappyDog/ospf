use std::{
    collections::HashMap,
    mem::size_of,
    net,
    sync::{Arc, Mutex},
};

use handle::recv_tcp_packet_raw_handle;
use pnet::{
    datalink,
    packet::{
        ip::IpNextHeaderProtocols::{Tcp, Udp},
        ipv4::Ipv4Packet,
    },
    transport::{self, TransportReceiver, TransportSender},
};
use tokio::{sync::broadcast, task::JoinHandle, time};

use crate::{
    interface, ipv4_addr_to_bits,
    neighbor::Neighbor,
    packet::{hello::HELLO_PACKET_TYPE, try_get_from_ipv4_packet, OspfPacket, OspfPacketHeader},
    prompt_and_read, router, AllSPFRouters, OSPF_VERSION_2,
};
pub mod event;
pub mod handle;
pub mod status;

#[derive(Clone, Copy)]
pub enum InterfaceNetworkType {
    Broadcast,
    PointToPoint,
    NBMA,
    PointToMultipoint,
    VirtualLink,
}

impl std::fmt::Display for InterfaceNetworkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterfaceNetworkType::Broadcast => write!(f, "Broadcast"),
            InterfaceNetworkType::PointToPoint => write!(f, "PointToPoint"),
            InterfaceNetworkType::NBMA => write!(f, "NBMA"),
            InterfaceNetworkType::PointToMultipoint => write!(f, "PointToMultipoint"),
            InterfaceNetworkType::VirtualLink => write!(f, "VirtualLink"),
        }
    }
}

impl std::fmt::Debug for InterfaceNetworkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterfaceNetworkType::Broadcast => write!(f, "Broadcast"),
            InterfaceNetworkType::PointToPoint => write!(f, "PointToPoint"),
            InterfaceNetworkType::NBMA => write!(f, "NBMA"),
            InterfaceNetworkType::PointToMultipoint => write!(f, "PointToMultipoint"),
            InterfaceNetworkType::VirtualLink => write!(f, "VirtualLink"),
        }
    }
}

pub struct Interface {
    pub name: String,
    pub ip_addr: net::Ipv4Addr,
    pub network_mask: net::Ipv4Addr,
    pub aread_id: net::Ipv4Addr,
    pub output_cost: u32,
    pub rxmt_interval: u32,
    pub inf_trans_delay: u32,
    pub router_priority: u32,
    pub hello_interval: u32,
    pub router_dead_interval: u32,
    pub auth_type: u32,
    pub network_type: InterfaceNetworkType,
    pub status: status::InterfaceStatus,
}

pub const DEFAULT_HELLO_INTERVAL: u32 = 10;
pub const DEFAULT_OUTPUT_COST: u32 = 1;
pub const DEFAULT_RXMT_INTERVAL: u32 = 5;
pub const DEFAULT_INF_TRANS_DELAY: u32 = 1;
pub const DEFAULT_ROUTER_PRIORITY: u32 = 1;
pub const DEFAULT_ROUTER_DEAD_INTERVAL: u32 = 40;
pub const DEFAULT_AUTH_TYPE: u32 = 0;
pub const DEFAULT_AUTH_KEY: u64 = 0;
pub const DEFAULT_AREA_ID: u32 = 0;

fn is_valid_pnet_interface(pnet_int: &datalink::NetworkInterface) -> bool {
    if pnet_int.is_loopback() || !pnet_int.is_up() {
        return false;
    }
    for ip in &pnet_int.ips {
        if let net::IpAddr::V4(_) = ip.ip() {
            if let net::IpAddr::V4(_) = ip.mask() {
                return true;
            }
        }
    }
    false
}

pub fn turn_on_interface(interface_name : &str) {
    let interfaces = crate::INTERFACES.lock().unwrap();
    for (_,interface) in interfaces.iter() {
        if interface.name == interface_name {
            
            break;
        }
    }
}


pub fn turn_down_interface(interface_name : &str) {
    let interfaces = crate::INTERFACES.lock().unwrap();
    for (_,interface) in interfaces.iter() {
        if interface.name == interface_name {
            
            break;
        }
    }
}

pub fn list_interfaces() {
    let interfaces = crate::INTERFACES.lock().unwrap();
    for (_,interface) in interfaces.iter() {
        println!("interface: {}\t{}\t{}",interface.name,interface.ip_addr,interface.network_mask);
    }
}

pub fn display_interface(interface_name : &str) {
    let interfaces = crate::INTERFACES.lock().unwrap();
    for (_,interface) in interfaces.iter() {
        if interface.name == interface_name {
            println!("interface: {}",interface.name);
            println!("ip address: {}",interface.ip_addr);
            println!("network mask: {}",interface.network_mask);
            println!("area id: {}",interface.aread_id);
            println!("output cost: {}",interface.output_cost);
            println!("rxmt interval: {}",interface.rxmt_interval);
            println!("status: {:#?}",interface.status);
            break;
        }
    }
}

pub fn get_ipv4_addr_mask_from_pnet_interface(
    pnet_int: &datalink::NetworkInterface,
) -> Option<(net::Ipv4Addr, net::Ipv4Addr)> {
    for ip in &pnet_int.ips {
        if let net::IpAddr::V4(addr) = ip.ip() {
            if let net::IpAddr::V4(mask) = ip.mask() {
                return Some((addr, mask));
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
    None
}

pub struct InterfaceTransmission {
    pub inner_tx: broadcast::Sender<Arc<Mutex<bytes::Bytes>>>,
    pub inner_rx: broadcast::Receiver<Arc<Mutex<bytes::Bytes>>>,
    pub tcp_tx : transport::TransportSender,
    pub tcp_rx : transport::TransportReceiver,
    pub udp_tx : transport::TransportSender,
    pub udp_rx : transport::TransportReceiver,
}


pub fn init_interface_tx_rx(interface_addrs : Vec<net::IpAddr>) -> Arc<Mutex<HashMap<net::IpAddr,InterfaceTransmission>>>  {
    let mut transmissions = HashMap::new();
    for addr in interface_addrs {
        let (inner_tx, inner_rx) = broadcast::channel::<Arc<Mutex<bytes::Bytes>>>(128);
        let (tcp_tx, tcp_rx) = transport::transport_channel(1500, transport::TransportChannelType::Layer3(Tcp)).unwrap();
        let (udp_tx, udp_rx) = transport::transport_channel(1500, transport::TransportChannelType::Layer3(Udp)).unwrap();
        transmissions.insert(addr, InterfaceTransmission {
            inner_tx,
            inner_rx,
            tcp_tx,
            tcp_rx,
            udp_tx,
            udp_rx,
        });
    }
    Arc::new(Mutex::new(transmissions))
}

pub fn init_interfaces() -> HashMap<net::IpAddr, Interface> {
    let mut interfaces = HashMap::new();
    let raw_interfaces = datalink::interfaces();
    for int in raw_interfaces {
        if !is_valid_pnet_interface(&int) {
            continue;
        }
        let (ip_addr, network_mask) = match get_ipv4_addr_mask_from_pnet_interface(&int) {
            Some((ip, mask)) => (ip, mask),
            None => continue,
        };
        println!("------------------------------------------");
        println!("detect interface: {}", int.name);
        let area_id: u32 = prompt_and_read(&format!(
            "please input the area id(default is {},press enter):",
            DEFAULT_AREA_ID
        ))
        .parse()
        .unwrap_or(DEFAULT_AREA_ID);
        let output_cost = prompt_and_read(&format!(
            "please input the output cost(default is {}):",
            DEFAULT_OUTPUT_COST
        ))
        .parse()
        .unwrap_or(DEFAULT_OUTPUT_COST);
        let rxmt_interval = prompt_and_read(&format!(
            "please input the rxmt interval(default is {}):",
            DEFAULT_RXMT_INTERVAL
        ))
        .parse()
        .unwrap_or(DEFAULT_RXMT_INTERVAL);
        let inf_trans_delay = prompt_and_read(&format!(
            "please input the inf_trans_delay(default is {}):",
            DEFAULT_INF_TRANS_DELAY
        ))
        .parse()
        .unwrap_or(DEFAULT_INF_TRANS_DELAY);
        let router_priority = prompt_and_read(&format!(
            "please input the router priority(default is {}):",
            DEFAULT_ROUTER_PRIORITY
        ))
        .parse()
        .unwrap_or(DEFAULT_ROUTER_PRIORITY);
        let hello_interval = prompt_and_read(&format!(
            "please input the hello interval(default is {}):",
            DEFAULT_HELLO_INTERVAL
        ))
        .parse()
        .unwrap_or(DEFAULT_HELLO_INTERVAL);
        let router_dead_interval = prompt_and_read(&format!(
            "please input the router dead interval(default is {}):",
            DEFAULT_ROUTER_DEAD_INTERVAL
        ))
        .parse()
        .unwrap_or(DEFAULT_ROUTER_DEAD_INTERVAL);
        let auth_type = prompt_and_read(&format!(
            "please input the auth type(default is {}):",
            DEFAULT_AUTH_TYPE
        ))
        .parse()
        .unwrap_or(DEFAULT_AUTH_TYPE);
        let network_type = if int.is_point_to_point() {
            InterfaceNetworkType::PointToPoint
        } else if int.is_multicast() {
            InterfaceNetworkType::PointToMultipoint
        } else if int.is_broadcast() {
            InterfaceNetworkType::Broadcast
        } else {
            crate::error("Unknown network type,skipped this interface.");
            continue;
        };
        interfaces.insert(
            net::IpAddr::V4(ip_addr),
            Interface {
                name: int.name.clone(),
                ip_addr: ip_addr,
                network_mask,
                aread_id: area_id.into(),
                output_cost,
                rxmt_interval,
                inf_trans_delay,
                router_priority,
                hello_interval,
                router_dead_interval,
                auth_type,
                network_type,
                status: status::InterfaceStatus::Down,
            },
        );
    }
    interfaces
}

impl Interface {
    pub const INNER_PACKET_QUEUE_SIZE: u32 = 128;

    pub fn get_area_id(&self) -> net::Ipv4Addr {
        self.aread_id
    }

    pub fn set_hello_interval(&mut self, hello_interval: u32) {
        self.hello_interval = hello_interval;
    }
    pub fn set_router_priority(&mut self, router_prioriry: u32) {
        self.router_priority = router_prioriry;
    }
    pub fn set_inf_trans_delay(&mut self, inf_trans_delay: u32) {
        self.inf_trans_delay = inf_trans_delay;
    }
    pub fn set_rxmt_interval(&mut self, rxmt_interval: u32) {
        self.rxmt_interval = rxmt_interval;
    }

   
}
