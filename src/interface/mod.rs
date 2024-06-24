pub mod event;
pub mod handle;
pub mod status;
pub mod trans;
use core::net;
use pnet::datalink;
use std::{collections::HashMap, fmt::Debug, sync::Arc};
use tokio::{
    sync::{Mutex, RwLock},
    task::JoinHandle,
};

use crate::{neighbor, util, ROUTER_ID};

pub const DEFAULT_HELLO_INTERVAL: u16 = 10;
pub const DEFAULT_OUTPUT_COST: u32 = 1;
pub const DEFAULT_RXMT_INTERVAL: u32 = 5;
pub const DEFAULT_INF_TRANS_DELAY: u32 = 1;
pub const DEFAULT_ROUTER_PRIORITY: u8 = 1;
pub const DEFAULT_ROUTER_DEAD_INTERVAL: u32 = 40;
pub const DEFAULT_AUTH_TYPE: u8 = 0;
pub const DEFAULT_AUTH_KEY: u64 = 0;
pub const DEFAULT_AREA_ID: u32 = 0;

pub enum NetworkType {
    Broadcast,
    PointToPoint,
    NBMA,
    PointToMultipoint,
    VirtualLink,
}

impl Debug for NetworkType {
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
    pub area_id: u32,
    pub output_cost: u32,
    pub rxmt_interval: u32,
    pub inf_trans_delay: u32,
    pub hello_interval: u16,
    pub router_dead_interval: u32,
    pub network_type: NetworkType,
    pub auth_type: u8,
    pub autn_key: u64,
}

impl Interface {}

lazy_static::lazy_static! {
    pub static ref INTERFACES : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Interface>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref INTERFACES_BY_NAME : Arc<RwLock<HashMap<String,Arc<RwLock<Interface>>>>> = Arc::new(RwLock::new(HashMap::new()));
}

async fn init_interfaces(interfaces: Vec<datalink::NetworkInterface>) {
    let mut interfaces_map = INTERFACES.write().await;
    let mut interfaces_name_map = INTERFACES_BY_NAME.write().await;
    interfaces.iter().for_each(|int| {
        let result: Option<(net::Ipv4Addr, net::Ipv4Addr)> =
            int.ips.iter().find(|ip| ip.is_ipv4()).map(|ip| {
                (
                    ip.ip().to_string().parse().unwrap(),
                    ip.mask().to_string().parse().unwrap(),
                )
            });
        if let Some((ip, mask)) = result {
            println!("--------------------------------");
            println!("Found interface {}", int.name);
            let area_id = util::prompt_and_read(&format!(
                "Enter the area id for interface {} (default is {}):",
                ip, DEFAULT_AREA_ID
            ))
            .parse::<u32>()
            .unwrap_or(DEFAULT_AREA_ID);
            let output_cost = util::prompt_and_read(&format!(
                "Enter the output cost for interface {} (default is {}):",
                ip, DEFAULT_OUTPUT_COST
            ))
            .parse::<u32>()
            .unwrap_or(DEFAULT_OUTPUT_COST);
            let rxmt_interval = util::prompt_and_read(&format!(
                "Enter the rxmt interval for interface {} (default is {}):",
                ip, DEFAULT_RXMT_INTERVAL
            ))
            .parse::<u32>()
            .unwrap_or(DEFAULT_RXMT_INTERVAL);
            let inf_trans_delay = util::prompt_and_read(&format!(
                "Enter the inf trans delay for interface {} (default is {}):",
                ip, DEFAULT_INF_TRANS_DELAY
            ))
            .parse::<u32>()
            .unwrap_or(DEFAULT_INF_TRANS_DELAY);
            let hello_interval = util::prompt_and_read(&format!(
                "Enter the hello interval for interface {} (default is {}):",
                ip, DEFAULT_HELLO_INTERVAL
            ))
            .parse::<u16>()
            .unwrap_or(DEFAULT_HELLO_INTERVAL);
            let router_dead_interval = util::prompt_and_read(&format!(
                "Enter the router dead interval for interface {} (default is {}):",
                ip, DEFAULT_ROUTER_DEAD_INTERVAL
            ))
            .parse::<u32>()
            .unwrap_or(DEFAULT_ROUTER_DEAD_INTERVAL);
            let network_type = loop {
                let network_type = util::prompt_and_read(&format!(
                    "Enter the network type for interface {} (default is Broadcast):",
                    ip
                ));
                match network_type.as_str() {
                    "Broadcast" => break NetworkType::Broadcast,
                    "PointToPoint" => break NetworkType::PointToPoint,
                    "NBMA" => break NetworkType::NBMA,
                    "PointToMultipoint" => break NetworkType::PointToMultipoint,
                    "VirtualLink" => break NetworkType::VirtualLink,
                    _ => {
                        println!("Invalid network type, please enter again");
                        continue;
                    }
                };
            };
            let auth_type = util::prompt_and_read(&format!(
                "Enter the auth type for interface {} (default is {}):",
                ip, DEFAULT_AUTH_TYPE
            ))
            .parse::<u8>()
            .unwrap_or(DEFAULT_AUTH_TYPE);
            let auth_key = util::prompt_and_read(&format!(
                "Enter the auth key for interface {} (default is {} if auth_type is not 0):",
                ip, DEFAULT_AUTH_KEY
            ))
            .parse::<u64>()
            .unwrap_or(DEFAULT_AUTH_KEY);
            let wrapped_interface = Arc::new(RwLock::new(Interface {
                ip,
                mask,
                area_id,
                output_cost,
                rxmt_interval,
                inf_trans_delay,
                hello_interval,
                router_dead_interval,
                network_type,
                auth_type,
                autn_key: auth_key,
            }));
            interfaces_name_map.insert(int.name.clone(), wrapped_interface.clone());
            interfaces_map.insert(ip, wrapped_interface.clone());
        }
    });
}

/// this function will init all the global data about interface
/// like the interfaces, neighbors, handlers
pub async fn init() -> Result<(), Box<dyn std::error::Error>> {
    util::debug(&format!("Router id is set to: {}", ROUTER_ID.clone()));
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
        tokio::spawn(handle::init_handlers(ipv4_addrs.clone())),
        tokio::spawn(trans::init_transmissions(ipv4_addrs.clone())),
        tokio::spawn(neighbor::init_neighbors(ipv4_addrs.clone())),
        tokio::spawn(init_interfaces(interfaces.clone())),
    )?;
    Ok(())
}

pub async fn status_changed(interface_name: String, event: event::Event) {
    match event {
        event::Event::InterfaceUp => {}
        event::Event::InterfaceDown => {}
        event::Event::LoopInd => {}
        event::Event::UnloopInd => {}
        event::Event::WaitTimer => {}
        event::Event::NeighborChange => {}
        event::Event::BackupSeen => {}
        _ => {
            util::error("Invalid event type,ignored.");
        }
    }
}

impl std::fmt::Display for Interface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Interface: {}\nMask: {}\nArea id: {}\nOutput cost: {}\nRxmt interval: {}\ninf_trans_delay:{}\nhello_interval:{}\nrouter_dead_interval:{}\nNetwork type: {:?}\nAuth type: {}\nAuth key: {}",
            self.ip, self.mask, self.area_id, self.output_cost, self.rxmt_interval
            ,self.inf_trans_delay,self.hello_interval,self.router_dead_interval,self.network_type,self.auth_type,self.autn_key
        )
    }
}

pub async fn interface_display(interface_name: String) {
    let interface_name_map = INTERFACES_BY_NAME.read().await;
    if let Some(interface) = interface_name_map.get(&interface_name) {
        let interface = interface.read().await;
        println!("\n--------------------------------");
        println!("Interface: {}", interface_name);
        println!("{}", interface);
    } else {
        util::error(&format!("Interface {} not found", interface_name));
    }
}

pub async fn interface_list() {
    let interface_name_map = INTERFACES_BY_NAME.read().await;

    for (name, interface) in interface_name_map.iter() {
        let interface = interface.read().await;
        println!("\n--------------------------------");
        println!("Interface: {}", name);
        println!("{}", interface);
    }
}
