use std::{
    collections::HashMap,
    mem::size_of,
    net,
    sync::{Arc, Mutex},
};

use handle::recv_tcp_packet_raw_handle;
use pnet::{
    datalink,
    packet::ip::IpNextHeaderProtocols::{Tcp, Udp},
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
    pub auth_key: u64,
    pub network_type: InterfaceNetworkType,
    pub trans_rx: TransportReceiver,
    pub trans_tx: TransportSender,
    pub inner_rx: broadcast::Receiver<Arc<Mutex<dyn crate::packet::OspfPacket + Send>>>,
    pub inner_tx: broadcast::Sender<Arc<Mutex<dyn crate::packet::OspfPacket + Send>>>,
    pub send_packet_handle: Option<JoinHandle<()>>,
    pub recv_packet_handle: Option<JoinHandle<()>>,
    pub produce_hello_packet_handle: Option<JoinHandle<()>>,
    pub produce_dd_packet_handle: Option<JoinHandle<()>>,
    pub neighbors: Arc<Mutex<HashMap<net::Ipv4Addr, Neighbor>>>,
    pub router: Arc<Mutex<crate::router::Router>>,
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

fn detect_pnet_interface() -> Result<Vec<datalink::NetworkInterface>, &'static str> {
    let interfaces = datalink::interfaces();
    if interfaces.len() == 0 {
        return Err("No interface found");
    }
    Ok(interfaces)
}

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

pub fn create_interfaces<'a>(
    router: Arc<Mutex<router::Router>>,
) -> Result<HashMap<String, Arc<Mutex<interface::Interface>>>, &'static str> {
    let pnet_ints = detect_pnet_interface()?;
    let mut ints = HashMap::new();
    for int in pnet_ints {
        if !is_valid_pnet_interface(&int) {
            continue;
        }
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
        let auth_key = prompt_and_read(&format!(
            "please input the auth key(default is {},PS: just for test):",
            DEFAULT_AUTH_KEY
        ))
        .parse()
        .unwrap_or(DEFAULT_AUTH_KEY);

        if let Some(int) = Interface::from_pnet_interface(
            router.clone(),
            &int,
            net::Ipv4Addr::from(area_id),
            output_cost,
            rxmt_interval,
            inf_trans_delay,
            router_priority,
            hello_interval,
            router_dead_interval,
            auth_type,
            auth_key,
        ) {
            ints.insert(int.name.clone(), Arc::new(Mutex::new(int)));
        }
    }
    println!("----------------- all interfaces are set,adding to router -------");
    Ok(ints)
}

impl Interface {
    pub const INNER_PACKET_QUEUE_SIZE: u32 = 128;

    pub fn update_neighbors(&mut self, new_neighbors: Arc<Mutex<HashMap<net::Ipv4Addr, Neighbor>>>) {
        let mut locked_neighbors = self.neighbors.lock().unwrap();
        let new_neighbors = new_neighbors.lock().unwrap();
        for (k, v) in new_neighbors.iter() {
            // locked_neighbors.insert(*k, v.clone());
        }
    }

    pub fn get_neighbors(&self) -> Arc<Mutex<HashMap<net::Ipv4Addr, Neighbor>>> {
        self.neighbors.clone()
    }

    pub fn get_area_id(&self) -> net::Ipv4Addr {
        self.aread_id
    }
    /// init the interfaces' handlers
    pub async fn init_handlers(
        &mut self,
        router_id: net::Ipv4Addr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let hello_interval = self.hello_interval as u16;
        let options = 0;
        let area_id = self.aread_id;
        let network_mask = self.network_mask;
        let router_priority = self.router_priority as u8;
        let router_dead_interval = self.router_dead_interval;
        let neighbors = self.neighbors.clone();

        let (udp_tx, udp_rx) =
            transport::transport_channel(1500, transport::TransportChannelType::Layer3(Udp))
                .unwrap();
        let (tcp_tx, tcp_rx) =
            transport::transport_channel(1500, transport::TransportChannelType::Layer3(Tcp))
                .unwrap();
        let (send_tcp_tx, send_tcp_rx) = broadcast::channel::<bytes::Bytes>(128);
        let (send_udp_tx, send_udp_rx) = broadcast::channel::<bytes::Bytes>(128);
        tokio::spawn(handle::send_tcp_packet_raw_handle(send_tcp_rx, tcp_tx));
        tokio::spawn(handle::send_udp_packet_raw_handle(send_udp_rx, udp_tx));
        tokio::spawn(handle::recv_tcp_packet_raw_handle(
            send_tcp_tx.clone(),
            tcp_rx,
        ));
        tokio::spawn(handle::recv_udp_packet_raw_handle(
            send_udp_tx.clone(),
            udp_rx,
        ));
        tokio::spawn(handle::create_hello_packet_raw_handle(
            send_udp_tx.clone(),
            hello_interval,
            network_mask,
            options,
            router_id.into(),
            area_id.into(),
            router_priority,
            router_dead_interval,
            0,
            0,
            self.ip_addr,
            AllSPFRouters,
            neighbors,
        ));
        Ok(())
    }
    pub fn from_pnet_interface(
        router: Arc<Mutex<router::Router>>,
        pnet_int: &datalink::NetworkInterface,
        aread_id: net::Ipv4Addr,
        output_cost: u32,
        rxmt_interval: u32,
        inf_trans_delay: u32,
        router_prioriry: u32,
        hello_interval: u32,
        router_dead_interval: u32,
        auth_type: u32,
        auth_key: u64,
    ) -> Option<Self> {
        if pnet_int.is_loopback() || !pnet_int.is_up() {
            return None;
        }
        let mut ip_addr = net::Ipv4Addr::new(255, 255, 255, 255); //false addr
        let mut network_mask = net::Ipv4Addr::new(255, 255, 255, 255);
        let mut network_type = InterfaceNetworkType::Broadcast;
        let mut found_ip_flag = false;
        for ip in &pnet_int.ips {
            if let net::IpAddr::V4(taddr) = ip.ip() {
                if let net::IpAddr::V4(tmask) = ip.mask() {
                    ip_addr = taddr;
                    network_mask = tmask;
                    found_ip_flag = true;
                    if pnet_int.is_point_to_point() {
                        network_type = InterfaceNetworkType::PointToPoint;
                    } else if pnet_int.is_multicast() {
                        network_type = InterfaceNetworkType::PointToMultipoint;
                    } else if pnet_int.is_broadcast() {
                        network_type = InterfaceNetworkType::Broadcast;
                    } else {
                        network_type = InterfaceNetworkType::NBMA;
                    }
                    break;
                }
            }
        }
        if !found_ip_flag {
            return None;
        }
        let name = pnet_int.name.clone();
        println!("interface [{}] set.", name);
        println!("interface ipv4 addr: {}", ip_addr);
        println!("interface network mask: {}", network_mask);

        let (inner_tx, inner_rx) = broadcast::channel::<Arc<Mutex<dyn OspfPacket + Send>>>(
            Interface::INNER_PACKET_QUEUE_SIZE as usize,
        );
        let (trans_tx, trans_rx) =
            transport::transport_channel(1500, transport::TransportChannelType::Layer3(Udp))
                .unwrap();

        let int = Self::new(
            ip_addr,
            network_mask,
            aread_id,
            output_cost,
            rxmt_interval,
            inf_trans_delay,
            router_prioriry,
            hello_interval,
            router_dead_interval,
            auth_type,
            auth_key,
            name,
            network_type,
            inner_tx,
            inner_rx,
            trans_tx,
            trans_rx,
            router,
        );
        Some(int)
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

    pub fn new(
        ip_addr: net::Ipv4Addr,
        network_mask: net::Ipv4Addr,
        aread_id: net::Ipv4Addr,
        output_cost: u32,
        rxmt_interval: u32,
        inf_trans_delay: u32,
        router_prioriry: u32,
        hello_interval: u32,
        router_dead_interval: u32,
        auth_type: u32,
        auth_key: u64,
        name: String,
        network_type: InterfaceNetworkType,
        inner_tx: broadcast::Sender<Arc<Mutex<dyn crate::packet::OspfPacket + Send>>>,
        inner_rx: broadcast::Receiver<Arc<Mutex<dyn crate::packet::OspfPacket + Send>>>,
        trans_tx: transport::TransportSender,
        trans_rx: transport::TransportReceiver,
        router: Arc<Mutex<router::Router>>,
    ) -> Self {
        Self {
            name,
            ip_addr,
            network_mask,
            aread_id,
            output_cost,
            rxmt_interval,
            inf_trans_delay,
            router_priority: router_prioriry,
            hello_interval,
            router_dead_interval,
            auth_type,
            auth_key,
            network_type,
            send_packet_handle: None,
            recv_packet_handle: None,
            produce_dd_packet_handle: None,
            produce_hello_packet_handle: None,
            neighbors: Arc::new(Mutex::new(HashMap::new())),
            router: router,
            inner_tx,
            inner_rx,
            trans_rx,
            trans_tx,
            status: status::InterfaceStatus::Down,
        }
    }
}
