pub mod event;
pub mod status;

use std::net;

pub struct Interface {
    pub ip_addr: net::IpAddr,
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

impl Interface {
    pub fn new(
        ip_addr: net::IpAddr,
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
    ) -> Self {
        Self {
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
