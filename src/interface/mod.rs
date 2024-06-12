pub mod event;
pub mod status;
use pnet::datalink;
use pnet::ipnetwork::IpNetwork;
use std::net::{self, IpAddr, Ipv4Addr};

pub struct Interface {
    pub name: String,
    pub ip_addr: net::Ipv4Addr,
    pub network_mask: net::Ipv4Addr,
    pub aread_id: net::Ipv4Addr,
    pub output_cost: u32,
    pub rxmt_interval: u32,
    pub inf_trans_delay: u32,
    pub router_prioriry: u32,
    pub hello_interval: u32,
    pub router_dead_interval: u32,
    pub auth_type: u32,
    pub auth_key: u32,
}

pub fn detect_pnet_interface() -> Result<Vec<datalink::NetworkInterface>, &'static str> {
    let interfaces = datalink::interfaces();
    if interfaces.len() == 0 {
        return Err("No interface found");
    }
    Ok(interfaces)
}

impl Interface {
    pub fn from_pnet_interface(
        pnet_int: &datalink::NetworkInterface,
        aread_id: net::Ipv4Addr,
        output_cost: u32,
        rxmt_interval: u32,
        inf_trans_delay: u32,
        router_prioriry: u32,
        hello_interval: u32,
        router_dead_interval: u32,
        auth_type: u32,
        auth_key: u32,
    ) -> Option<Self> {
        let mut ip_addr = Ipv4Addr::new(255, 255, 255, 255); //false addr
        let mut network_mask = Ipv4Addr::new(255, 255, 255, 255);

        let mut found_ip_flag = false;
        for ip in &pnet_int.ips {
            if let IpAddr::V4(taddr) = ip.ip() {
                if let IpAddr::V4(tmask) = ip.mask() {
                    ip_addr = taddr;
                    network_mask = tmask;
                    found_ip_flag = true;
                    break;
                }
            }
        }
        if !found_ip_flag {
            return None;
        }
        let name = pnet_int.name.clone();
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
        );
        Some(int)
    }
    pub fn set_hello_interval(&mut self, hello_interval: u32) {
        self.hello_interval = hello_interval;
    }
    pub fn set_router_priority(&mut self, router_prioriry: u32) {
        self.router_prioriry = router_prioriry;
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
        auth_key: u32,
        name: String,
    ) -> Self {
        Self {
            name,
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
        }
    }
}
