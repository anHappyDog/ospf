use crate::interface;
use crate::lsa::network::NetworkLinkStateAdvertisement;
use crate::lsa::LinkStateAdvertisement;
use crate::rtable;
use std::collections::HashMap;
use std::net;

pub struct Router {
    router_table: Vec<rtable::RouteTable>,
    router_id: net::Ipv4Addr,
    area_id: net::Ipv4Addr,
    interfaces: HashMap<String, interface::Interface>,
}

impl Router {
    pub fn new(router_id: net::Ipv4Addr, area_id: net::Ipv4Addr) -> Self {
        Self {
            router_table: Vec::new(),
            interfaces: HashMap::new(),
            router_id,
            area_id,
        }
    }
    pub fn add_interface(&mut self, interface: interface::Interface) {

    }
}
