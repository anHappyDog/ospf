use std::{
    collections::HashMap,
    mem::size_of,
    net,
    sync::{Arc, Mutex},
};

use handle::recv_tcp_packet;
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
    lsa::router,
    neighbor::Neighbor,
    packet::{hello::HELLO_PACKET_TYPE, try_get_from_ipv4_packet, OspfPacket, OspfPacketHeader},
    prompt_and_read, AllSPFRouters, OSPF_VERSION_2,
};
pub mod event;
pub mod handle;
pub mod status;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
    pub router_priority: u8,
    pub hello_interval: u8,
    pub router_dead_interval: u32,
    pub auth_type: u32,
    pub network_type: InterfaceNetworkType,
    pub status: status::InterfaceStatus,
    pub designated_router: net::Ipv4Addr,
    pub backup_designated_router: net::Ipv4Addr,
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

pub fn turn_on_interface(interface_name: &str) {
    let mut interfaces = crate::INTERFACES.lock().unwrap();
    for (_, interface) in interfaces.iter_mut() {
        if interface.name == interface_name {
            interface.change_to(event::InterfaceEvent::InterfaceUp);
            break;
        }
    }
}

pub fn turn_down_interface(interface_name: &str) {
    let interfaces = crate::INTERFACES.lock().unwrap();
    for (_, interface) in interfaces.iter() {
        if interface.name == interface_name {
            break;
        }
    }
}

pub fn list_interfaces() {
    let interfaces = crate::INTERFACES.lock().unwrap();
    for (_, interface) in interfaces.iter() {
        let locked_interface = interface.lock().unwrap();
        println!(
            "interface: {}\t{}\t{}",
            locked_interface.name, locked_interface.ip_addr, locked_interface.network_mask
        );
    }
}

pub fn display_interface(interface_name: &str) {
    let interfaces = crate::INTERFACES.lock().unwrap();

    for (_, interface) in interfaces.iter() {
        let locked_interface = interface.lock().unwrap();
        if locked_interface.name == interface_name {
            println!("interface: {}", locked_interface.name);
            println!("ip address: {}", locked_interface.ip_addr);
            println!("network mask: {}", locked_interface.network_mask);
            println!("area id: {}", locked_interface.aread_id);
            println!("output cost: {}", locked_interface.output_cost);
            println!("rxmt interval: {}", locked_interface.rxmt_interval);
            println!("status: {:#?}", locked_interface.status);
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

// these are used for the inter- interface transmission like forwarding packets.
// when some packet needs forwarding, just use the according interfaces' rx then send it.
pub struct InterfaceTransmission {
    pub tcp_inner_tx: broadcast::Sender<bytes::Bytes>,
    pub tcp_inner_rx: broadcast::Receiver<bytes::Bytes>,
    pub udp_inner_tx: broadcast::Sender<bytes::Bytes>,
    pub udp_inner_rx: broadcast::Receiver<bytes::Bytes>,
}

pub fn init_interface_tx_rx(
    interface_addrs: Vec<net::IpAddr>,
) -> Arc<Mutex<HashMap<net::IpAddr, InterfaceTransmission>>> {
    let mut transmissions = HashMap::new();
    for addr in interface_addrs {
        let (tcp_inner_tx, tcp_inner_rx) = broadcast::channel::<bytes::Bytes>(128);
        let (udp_inner_tx, udp_inner_rx) = broadcast::channel::<bytes::Bytes>(128);

        transmissions.insert(
            addr,
            InterfaceTransmission {
                tcp_inner_tx,
                tcp_inner_rx,
                udp_inner_tx,
                udp_inner_rx,
            },
        );
    }
    Arc::new(Mutex::new(transmissions))
}

pub fn init_interfaces() -> HashMap<net::IpAddr, Arc<Mutex<Interface>>> {
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
            Arc::new(Mutex::new(Interface {
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
            })),
        );
    }
    interfaces
}

impl Interface {
    pub const INNER_PACKET_QUEUE_SIZE: u32 = 128;

    pub fn get_area_id(&self) -> net::Ipv4Addr {
        self.aread_id
    }

    fn interface_up(&mut self) {
        if self.router_dead_interval == 0 {
            crate::error("router dead interval is zero, cannot change to waiting.");
            return;
        }
        if self.hello_interval == 0 {
            crate::error("hello interval is zero, cannot change to waiting.");
            return;
        }
        let hello_interval = self.hello_interval;
        let router_dead_interval = self.router_dead_interval;
        let interface_name = self.name.clone();
        let router_priority = self.router_priority;
        let interface_transmission = crate::INTERFACE_TRANSMISSION.clone();
        let interface_neighbors = crate::NEIGHBORS.clone();
        let locked_interface_neighbors = interface_neighbors
            .lock()
            .expect("lock interface neighbors failed.");
        let locked_interface_transmission = interface_transmission
            .lock()
            .expect("lock interface transmission failed.");
        let neighbors = locked_interface_neighbors
            .get(&self.ip_addr.into())
            .expect("get interface neighbors failed.");
        let trans = locked_interface_transmission
            .get(&self.ip_addr.into())
            .expect("get interface transmission failed.");
        let udp_inner_tx = trans.udp_inner_tx.clone();
        let tcp_inner_tx = trans.tcp_inner_tx.clone();
        let (tcp_tx, tcp_rx) =
            transport::transport_channel(crate::MTU, transport::TransportChannelType::Layer3(Tcp))
                .expect("tcp channel error");
        let (udp_tx, udp_rx) =
            transport::transport_channel(crate::MTU, transport::TransportChannelType::Layer3(Udp))
                .expect("udp channel error");
        let handlers = handle::add_interface_handlers(self.ip_addr.into());
        let mut locked_handlers = handlers.lock().unwrap();
        locked_handlers.send_tcp_packet_handle = Some(tokio::spawn(handle::send_tcp_packet(
            tcp_inner_tx.subscribe(),
            tcp_tx,
        )));
        locked_handlers.send_udp_packet_handle = Some(tokio::spawn(handle::send_udp_packet(
            udp_inner_tx.subscribe(),
            udp_tx,
        )));
        locked_handlers.recv_tcp_packet_handle =
            Some(tokio::spawn(recv_tcp_packet(tcp_inner_tx.clone(), tcp_rx)));
        locked_handlers.recv_udp_packet_handle = Some(tokio::spawn(handle::recv_udp_packet(
            udp_inner_tx.clone(),
            udp_rx,
        )));
        crate::debug(&format!(
            "interface {} turns on hello timer.",
            interface_name
        ));
        locked_handlers.hello_timer_handle = Some(tokio::spawn(handle::create_hello_packet(
            self.ip_addr.into(),
        )));
        // self.hello_timer = Some(tokio::spawn(handle::create_hello_packet(
        //     udp_inner_tx.clone(),
        //     hello_interval as u16,
        //     network_mask,
        //     options,
        //     router_id.into(),
        //     area_id.into(),
        //     router_priority as u8,
        //     router_dead_interval,
        //     designated_router,
        //     backup_designated_router,
        //     src_ip,
        //     dst_ip,
        //     neighbors.clone(),
        // )));
        if router_priority == 0 {
            self.status = status::InterfaceStatus::DRother;
            crate::debug(&format!(
                "interface {}changed to status: DRother",
                interface_name
            ));
            return;
        }
        match self.network_type {
            InterfaceNetworkType::PointToPoint
            | InterfaceNetworkType::PointToMultipoint
            | InterfaceNetworkType::VirtualLink => {
                crate::debug(&format!(
                    "interface {}changed to status: PointToPoint",
                    interface_name
                ));
                self.status = status::InterfaceStatus::PointToPoint;
            }
            InterfaceNetworkType::Broadcast | InterfaceNetworkType::NBMA => {
                crate::debug(&format!("interface {} turns on wait timer.", self.name));
                // self.wait_timer = Some(tokio::spawn(handle::create_wait_timer(
                //     router_dead_interval,
                // )));
                self.status = status::InterfaceStatus::Waiting;
                crate::debug(&format!(
                    "interface {} changed to status: Waiting",
                    self.name
                ));
                if self.network_type == InterfaceNetworkType::NBMA {
                    // should send Start event to the neighbors who can be DR.
                }
            }
        }
    }

    pub fn change_to(&mut self, event: event::InterfaceEvent) {
        match event {
            event::InterfaceEvent::LoopInd => {
                // here may be wrong
                if self.status != status::InterfaceStatus::Down {
                    crate::error("LoopInd event received on non-down interface.");
                    return;
                }
                self.status = status::InterfaceStatus::Loopback;
            }
            event::InterfaceEvent::UnloopInd => {
                if self.status != status::InterfaceStatus::Loopback {
                    crate::error("UnloopInd event received on non-loopback interface.");
                    return;
                }
                self.status = status::InterfaceStatus::Down;
            }
            event::InterfaceEvent::WaitTimer | event::InterfaceEvent::BackupSeen => {
                if self.status != status::InterfaceStatus::Waiting {
                    crate::error("BackupSeen event received on non-waiting interface.");
                    return;
                }
                self.status = status::InterfaceStatus::Question;
            }
            event::InterfaceEvent::NeighborChange(status) => {
                if self.status != status::InterfaceStatus::Question {
                    crate::error("NeighborChange event received on non-question interface.");
                    return;
                }
                match status {
                    status::InterfaceStatus::DR => {
                        self.status = status::InterfaceStatus::DRother;
                    }
                    status::InterfaceStatus::DRother => {
                        self.status = status::InterfaceStatus::DR;
                    }
                    status::InterfaceStatus::Backup => {
                        self.status = status::InterfaceStatus::DRother;
                    }
                    _ => {
                        crate::error("NeighborChange event received with invalid status.");
                    }
                }
            }
            event::InterfaceEvent::InterfaceUp => {
                if self.status != status::InterfaceStatus::Down {
                    crate::error("InterfaceUp event received on non-down interface.");
                    return;
                }
                self.interface_up();
            }
            event::InterfaceEvent::InterfaceDown => {
                //not implemented fully
                self.status = status::InterfaceStatus::Down;
            }
        }
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
