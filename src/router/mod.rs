use crate::rtable;
use crate::{debug, interface};
use std::collections::HashMap;
use std::net;
use std::sync::{Arc, Mutex};

pub struct Router {
    router_table: Vec<rtable::RouteTable>,
    router_id: net::Ipv4Addr,
    interfaces: HashMap<String, Arc<Mutex<interface::Interface>>>,
}


impl Router {
    pub const MAX_INNER_PACKET_QUEUE_SIZE: usize = 100;
    pub fn get_router_id(&self) -> net::Ipv4Addr {
        self.router_id
    }
    pub fn new(router_id: net::Ipv4Addr) -> Self {
        Self {
            router_table: Vec::new(),
            interfaces: HashMap::new(),
            router_id,
        }
    }
    pub fn add_interface(&mut self, name: String, interface: Arc<Mutex<interface::Interface>>) {
        self.interfaces.insert(name, interface);
    }
    pub fn add_interfaces(
        &mut self,
        interfaces: HashMap<String, Arc<Mutex<interface::Interface>>>,
    ) {
        self.interfaces = interfaces;
    }
}
