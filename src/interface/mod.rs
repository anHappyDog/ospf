pub mod event;
pub mod handle;
pub mod status;
pub mod trans;

use core::net;
use std::{collections::HashMap, net::Ipv4Addr, sync::Arc};

use pnet::{datalink, packet::ip};
use tokio::sync::RwLock;

use crate::{neighbor, ROUTER_ID};

lazy_static::lazy_static! {
    pub static ref INTERFACE_STATUS_MAP: Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<status::Status>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref INTERFACE_MAP: Arc<RwLock<HashMap<net::Ipv4Addr,Interface>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref NAME_MAP : Arc<RwLock<HashMap<String,net::Ipv4Addr>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub const DEFAULT_HELLO_INTERVAL: u16 = 10;
pub const DEFAULT_OUTPUT_COST: u32 = 1;
pub const DEFAULT_RXMT_INTERVAL: u32 = 5;
pub const DEFAULT_INF_TRANS_DELAY: u32 = 1;
pub const DEFAULT_ROUTER_PRIORITY: u8 = 1;
pub const DEFAULT_ROUTER_DEAD_INTERVAL: u32 = 40;
pub const DEFAULT_AUTH_TYPE: u16 = 0;
pub const DEFAULT_AUTH_KEY: u64 = 0;
pub const DEFAULT_AREA_ID: u32 = 0;
pub const DEFAULT_OPTIONS: u8 = 0;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetworkType {
    Broadcast,
    PointToPoint,
    NBMA,
    PointToMultipoint,
    VirtualLink,
}

unsafe impl Send for NetworkType {}

impl std::fmt::Debug for NetworkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkType::Broadcast => write!(f, "Broadcast"),
            NetworkType::PointToPoint => write!(f, "PointToPoint"),
            NetworkType::NBMA => write!(f, "NBMA"),
            NetworkType::PointToMultipoint => write!(f, "PointToMultipoint"),
            NetworkType::VirtualLink => write!(f, "VirtualLink"),
        }
    }
}

pub struct Interface {
    pub ip: net::Ipv4Addr,
    pub mask: net::Ipv4Addr,
    pub area_id: net::Ipv4Addr,
    pub output_cost: u32,
    pub rxmt_interval: u32,
    pub inf_trans_delay: u32,
    pub hello_interval: u16,
    pub router_dead_interval: u32,
    pub network_type: NetworkType,
    pub auth_type: u16,
    pub auth_key: u64,
    pub options: u8,
    pub router_priority: u8,
}

fn try_get_ip_mask(
    interface: &datalink::NetworkInterface,
) -> Option<(net::Ipv4Addr, net::Ipv4Addr)> {
    interface.ips.iter().find(|ip| ip.is_ipv4() && !interface.is_loopback() ).map(|ip| {
        let iaddr: Ipv4Addr = ip.ip().to_string().parse::<net::Ipv4Addr>().unwrap();
        let mask = ip.mask().to_string().parse::<net::Ipv4Addr>().unwrap();
        (iaddr, mask)
    })
}

pub async fn add(interface: &datalink::NetworkInterface) {
    let ip_mask = try_get_ip_mask(interface);
    if let None = ip_mask {
        return;
    }
    let (ip, mask) = ip_mask.unwrap();
    println!("--------------------------------");
    println!("Found interface {}", interface.name);
    let mut name_map = NAME_MAP.write().await;
    name_map.insert(interface.name.clone(), ip);
    drop(name_map);
    let area_id = crate::util::prompt_and_read(&format!(
        "Enter the area id for interface {} (default is {}):",
        ip, DEFAULT_AREA_ID
    ))
    .parse::<u32>()
    .unwrap_or(DEFAULT_AREA_ID);
    let output_cost = crate::util::prompt_and_read(&format!(
        "Enter the output cost for interface {} (default is {}):",
        ip, DEFAULT_OUTPUT_COST
    ))
    .parse::<u32>()
    .unwrap_or(DEFAULT_OUTPUT_COST);
    let rxmt_interval = crate::util::prompt_and_read(&format!(
        "Enter the rxmt interval for interface {} (default is {}):",
        ip, DEFAULT_RXMT_INTERVAL
    ))
    .parse::<u32>()
    .unwrap_or(DEFAULT_RXMT_INTERVAL);
    let inf_trans_delay = crate::util::prompt_and_read(&format!(
        "Enter the inf trans delay for interface {} (default is {}):",
        ip, DEFAULT_INF_TRANS_DELAY
    ))
    .parse::<u32>()
    .unwrap_or(DEFAULT_INF_TRANS_DELAY);
    let hello_interval = crate::util::prompt_and_read(&format!(
        "Enter the hello interval for interface {} (default is {}):",
        ip, DEFAULT_HELLO_INTERVAL
    ))
    .parse::<u16>()
    .unwrap_or(DEFAULT_HELLO_INTERVAL);
    let router_dead_interval = crate::util::prompt_and_read(&format!(
        "Enter the router dead interval for interface {} (default is {}):",
        ip, DEFAULT_ROUTER_DEAD_INTERVAL
    ))
    .parse::<u32>()
    .unwrap_or(DEFAULT_ROUTER_DEAD_INTERVAL);
    let network_type = loop {
        let network_type = crate::util::prompt_and_read(&format!(
            "Enter the network type for interface {} (default is Broadcast):",
            ip
        ));
        match network_type.as_str() {
            "Broadcast" => break NetworkType::Broadcast,
            "PointToPoint" => break NetworkType::PointToPoint,
            "NBMA" => break NetworkType::NBMA,
            "PointToMultipoint" => break NetworkType::PointToMultipoint,
            "VirtualLink" => break NetworkType::VirtualLink,
            _ => break NetworkType::Broadcast,
        };
    };
    let auth_type = crate::util::prompt_and_read(&format!(
        "Enter the auth type for interface {} (default is {}):",
        ip, DEFAULT_AUTH_TYPE
    ))
    .parse::<u16>()
    .unwrap_or(DEFAULT_AUTH_TYPE);
    let auth_key = crate::util::prompt_and_read(&format!(
        "Enter the auth key for interface {} (default is {} if auth_type is not 0):",
        ip, DEFAULT_AUTH_KEY
    ))
    .parse::<u64>()
    .unwrap_or(DEFAULT_AUTH_KEY);
    let router_priority = crate::util::prompt_and_read(&format!(
        "Enter the router priority for interface {} (default is {}):",
        ip, DEFAULT_ROUTER_PRIORITY
    ))
    .parse()
    .unwrap_or(DEFAULT_ROUTER_PRIORITY);
    let options = DEFAULT_OPTIONS;

    let int = Interface {
        ip,
        mask,
        area_id: net::Ipv4Addr::new(
            area_id as u8,
            (area_id >> 8) as u8,
            (area_id >> 16) as u8,
            (area_id >> 24) as u8,
        ),
        output_cost,
        rxmt_interval,
        inf_trans_delay,
        hello_interval,
        router_dead_interval,
        network_type,
        auth_type,
        auth_key,
        options,
        router_priority,
    };

    let mut interface_map = INTERFACE_MAP.write().await;
    interface_map.insert(ip, int);
    drop(interface_map);

    let mut interface_status_map = INTERFACE_STATUS_MAP.write().await;
    interface_status_map.insert(ip, Arc::new(RwLock::new(status::Status::Down)));
    drop(interface_status_map);

    crate::util::log(&format!("Interface {} added.", ip));
}

async fn init_interfaces(interfaces: Vec<datalink::NetworkInterface>) {
    for interface in interfaces {
        add(&interface).await;
    }
}

pub async fn init() -> Result<(), Box<dyn std::error::Error>> {
    crate::util::log(&format!("Router id is set to: {}", ROUTER_ID.clone()));
    let interfaces = pnet::datalink::interfaces();
    let ipv4_addrs: Vec<net::Ipv4Addr> = interfaces
        .iter()
        .map(|interface| {
            interface
                .ips
                .iter()
                .find(|ip| ip.is_ipv4())
                .unwrap()
                .ip()
                .to_string()
                .parse()
                .unwrap()
        })
        .collect();

    tokio::try_join!(
        tokio::spawn(init_interfaces(interfaces.clone())),
        tokio::spawn(handle::init(ipv4_addrs.clone())),
        tokio::spawn(trans::init(ipv4_addrs.clone()))
    )?;

    Ok(())
}

pub async fn get_status(iaddr: net::Ipv4Addr) -> status::Status {
    let status_map = INTERFACE_STATUS_MAP.read().await;
    let status = status_map.get(&iaddr).unwrap();
    let locked_status = status.read().await;
    locked_status.clone()
}

pub async fn set_status(iaddr: net::Ipv4Addr, status: status::Status) {
    let status_map = INTERFACE_STATUS_MAP.read().await;
    let int_status = status_map.get(&iaddr).unwrap();
    let mut locked_status = int_status.write().await;
    *locked_status = status;
}

pub async fn send_neighbor_killnbr(ip: net::Ipv4Addr) {
    let neighbor_map = neighbor::INT_NEIGHBORS_MAP.read().await;
    let int_neighbor_map = neighbor_map.get(&ip).unwrap();
    let locked_int_neighbor_map = int_neighbor_map.read().await;
    for neighbor_ip in locked_int_neighbor_map.iter() {
        let naddr: net::Ipv4Addr = net::Ipv4Addr::from(*neighbor_ip);
        neighbor::event::send(naddr, neighbor::event::Event::KillNbr).await;
    }
}

pub async fn list() {
    let interface_map = INTERFACE_MAP.read().await;
    for (ip, interface) in interface_map.iter() {
        println!("---------------------");
        println!("ip :{}", ip);
        println!("mask :{}", interface.mask);
        println!("area_id :{}", interface.area_id);
        println!("output_cost :{}", interface.output_cost);
        println!("rxmt_interval :{}", interface.rxmt_interval);
    }
}

pub async fn display(name: String) {}
