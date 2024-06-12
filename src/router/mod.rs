use crate::interface;
use crate::lsa::network::NetworkLinkStateAdvertisement;
use crate::lsa::LinkStateAdvertisement;
use crate::rtable;
use std::net;

pub struct Router {
    router_table: Vec<rtable::RouteTable>,
    router_id: net::Ipv4Addr,
    area_id: net::Ipv4Addr,
}

impl Router {
    pub fn new(router_id: net::Ipv4Addr, area_id: net::Ipv4Addr) -> Self {
        Self {
            router_table: Vec::new(),
            router_id,
            area_id,
        }
    }
}
