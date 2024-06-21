use colored::*;
use interface::InterfaceTransmission;
use neighbor::Neighbor;
use pnet::{
    packet::{
        ip::{
            IpNextHeaderProtocol,
            IpNextHeaderProtocols::{Tcp, Udp},
        },
        tcp::Tcp,
    },
    transport,
};
use std::{
    collections::HashMap,
    io::{stdin, stdout, Write},
    net,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
pub mod area;
pub mod r#as;
pub mod error;
pub mod interface;
pub mod lsa;
pub mod neighbor;
pub mod packet;
pub mod router;
pub mod rtable;

#[allow(non_upper_case_globals)]
pub const AllSPFRouters: net::Ipv4Addr = crate::bits_to_ipv4_addr(0xe0000005);
#[allow(non_upper_case_globals)]
pub const AllDRouters: net::Ipv4Addr = crate::bits_to_ipv4_addr(0xe0000006);

pub const OSPF_VERSION_2: u8 = 2;
pub const OSPF_IP_PROTOCOL_NUMBER: u8 = 89;
pub const MTU: usize = 1500;

pub fn prompt_and_read(prompt: &str) -> String {
    print!("{}", prompt);
    stdout().flush().unwrap();

    let mut input = String::new();
    stdin().read_line(&mut input).expect("read line error");

    input.trim().to_string()
}

pub fn debug(msg: &str) {
    println!("{}", format!("[debug]:{}", msg).yellow());
}

pub fn log(msg: &str) {
    println!("{}", format!("[log]:{}", msg).green());
}

pub fn error(msg: &str) {
    println!("{}", format!("[error]:{}", msg).red());
}

pub const fn bits_to_ipv4_addr(bits: u32) -> net::Ipv4Addr {
    net::Ipv4Addr::new(
        ((bits >> 24) & 0xff) as u8,
        ((bits >> 16) & 0xff) as u8,
        ((bits >> 8) & 0xff) as u8,
        (bits & 0xff) as u8,
    )
}

pub const fn ipv4_addr_to_bits(ip: net::Ipv4Addr) -> u32 {
    (ip.octets()[0] as u32) << 24
        | (ip.octets()[1] as u32) << 16
        | (ip.octets()[2] as u32) << 8
        | ip.octets()[3] as u32
}

fn input_router_id() -> net::Ipv4Addr {
    loop {
        match prompt_and_read("please enter router id:").parse::<net::Ipv4Addr>() {
            Ok(id) => {
                return id;
            }
            Err(_) => {
                println!("Invalid router id, please try again.");
            }
        }
    }
}

lazy_static::lazy_static! {
    pub static ref INTERFACES : Arc<Mutex<HashMap<net::IpAddr,interface::Interface>>> = Arc::new(Mutex::new(HashMap::new()));
    pub static ref INTERFACE_TRANSMISSION : Arc<Mutex<HashMap<net::IpAddr,InterfaceTransmission>>> = Arc::new(Mutex::new(HashMap::new()));
    pub static ref ROUTER_ID : net:: Ipv4Addr = input_router_id();
    pub static ref ROUTER_TABLE : Arc<Mutex<Vec<rtable::RouteTable>>> = Arc::new(Mutex::new(Vec::new()));
    pub static ref NEIGHBORS : Arc<Mutex<HashMap<net::Ipv4Addr,Arc<Mutex<HashMap<net::Ipv4Addr,Neighbor>>>>>> = Arc::new(Mutex::new(HashMap::new()));
}

pub fn init() {
    crate::debug(&format!("router id is {}", ROUTER_ID.clone()));
    let interfaces = INTERFACES.clone();
    let interface_transmission = INTERFACE_TRANSMISSION.clone();
    let mut locked_interfaces = interfaces.lock().expect("interface lock error");
    let detected_interfaces = interface::init_interfaces();
    locked_interfaces.extend(detected_interfaces);
    let mut locked_interface_transmission = interface_transmission
        .lock()
        .expect("interface transmission lock error");
    for (ip, _) in locked_interfaces.iter() {
        let (tcp_tx, tcp_rx) =
            transport::transport_channel(MTU, transport::TransportChannelType::Layer3(Tcp))
                .expect("tcp channel error");
        let (udp_tx, udp_rx) =
            transport::transport_channel(MTU, transport::TransportChannelType::Layer3(Udp))
                .expect("udp channel error");
        let (inner_tx, inner_rx) = broadcast::channel(128);
        locked_interface_transmission.insert(
            *ip,
            InterfaceTransmission {
                tcp_tx,
                tcp_rx,
                udp_tx,
                udp_rx,
                inner_tx,
                inner_rx,
            },
        );
    }
}
