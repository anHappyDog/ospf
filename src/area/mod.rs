use std::net;

use crate::{
    interface,
    lsa::{self},
};

pub struct AddressRange {
    start_ipaddr: net::IpAddr,
    end_ipaddr: net::IpAddr,
    network_mask: net::Ipv4Addr,
}

pub struct Area {
    area_id: net::Ipv4Addr,
    addr_range_list: Vec<AddressRange>,
    interface_list: Vec<interface::Interface>,
    router_lsa_list: Vec<lsa::router::RouterLinkStateAdvertisement>,
    network_lsa_list: Vec<lsa::network::NetworkLinkStateAdvertisement>,
    summary_lsa_list: Vec<lsa::summary::SummaryLinkStateAdvertisement>,
    short_path_tree: usize,
    transit_capabilty: bool,
    external_routing_capabilty: bool,
    stub_default_cost: u32,
}

impl Area {
    pub fn new(
        transit_capabilty: bool,
        external_routing_capabilty: bool,
        stub_default_cost: u32,
        area_id: net::Ipv4Addr,
        addr_range_list: Vec<AddressRange>,
    ) -> Self {
        Self {
            interface_list: Vec::new(),
            router_lsa_list: Vec::new(),
            network_lsa_list: Vec::new(),
            summary_lsa_list: Vec::new(),
            short_path_tree: 0,
            area_id,
            addr_range_list,
            transit_capabilty,
            external_routing_capabilty,
            stub_default_cost,
        }
    }
}
