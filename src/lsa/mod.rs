use core::net;
use std::{collections::HashMap, sync::Arc};

use pnet::util::Octets;
use tokio::sync::RwLock;

pub mod as_external;
pub mod network;
pub mod router;
pub mod summary;

pub const LS_REFRESH_TIME: u32 = 1800;
pub const LS_MIN_INTERVAL: u32 = 5;
pub const LS_MIN_ARRIVAL: u32 = 1;
pub const MAX_AGE: u32 = 3600;
pub const CHECK_AGE: u32 = 300;
pub const MAX_AGE_DIFF: u32 = 900;
pub const LS_INIFINITY: u32 = 0xffffff;
pub const DEFAULT_DESTINATION: net::Ipv4Addr = net::Ipv4Addr::new(0, 0, 0, 0);
pub const INITIAL_SEQUENCE_NUMBER: i32 = 0x80000001u32 as i32;
pub const MAX_SEQUENCE_NUMBER: i32 = 0x7fffffff;

#[derive(Clone, Copy)]
pub struct Header {
    pub age: u16,
    pub options: u8,
    pub lsa_type: u8,
    pub link_state_id: u32,
    pub advertising_router: u32,
    pub sequence_number: u32,
    pub checksum: u16,
    pub length: u16,
}

pub const OPTION_DN: u8 = 0b1000_0000;
pub const OPTION_OPAQUE: u8 = 0b0100_0000;
pub const OPTION_DEMAND_CIRCUIT: u8 = 0b0010_0000;
pub const OPTION_LLS_DATA_BLOCK: u8 = 0b0001_0000;
pub const OPTION_NSSA: u8 = 0b0000_1000;
pub const OPTION_MUTLICAST: u8 = 0b0000_0100;
pub const OPTION_EXTERNAL: u8 = 0b0000_0010;
pub const OPTION_MT: u8 = 0b0000_0001;

impl Header {
    pub fn length() -> usize {
        20
    }
    pub fn try_from_be_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::length() {
            return None;
        }
        let age = u16::from_be_bytes([bytes[0], bytes[1]]);
        let options = bytes[2];
        let lsa_type = bytes[3];
        let link_state_id = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let advertising_router = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let sequence_number = u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let checksum = u16::from_be_bytes([bytes[16], bytes[17]]);
        let length = u16::from_be_bytes([bytes[18], bytes[19]]);
        Some(Self {
            age,
            options,
            lsa_type,
            link_state_id,
            advertising_router,
            sequence_number,
            checksum,
            length,
        })
    }
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.age.to_be_bytes());
        bytes.extend_from_slice(&self.options.octets());
        bytes.extend_from_slice(&self.lsa_type.octets());
        bytes.extend_from_slice(&self.link_state_id.to_be_bytes());
        bytes.extend_from_slice(&self.advertising_router.to_be_bytes());
        bytes.extend_from_slice(&self.sequence_number.to_be_bytes());
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.length.to_be_bytes());
        bytes
    }
}

#[derive(Clone, Copy)]
pub struct LsaIdentifer {
    pub lsa_type: u8,
    pub link_state_id: u32,
    pub advertising_router: u32,
}

#[derive(Clone)]
pub enum LSA {
    Router(router::RouterLSA),
    Network(network::NetworkLSA),
    Summary(summary::SummaryLSA),
    ASExternal(as_external::ASExternalLSA),
}

lazy_static::lazy_static! {
    /// the area_id is the key
    pub static ref LSDB : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<LsaDb>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref LSDB2 : Arc<RwLock<HashMap<LsaIdentifer,Lsa>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub struct LsaDb {
    pub router_lsa: HashMap<LsaIdentifer, router::RouterLSA>,
    pub network_lsa: HashMap<LsaIdentifer, network::NetworkLSA>,
    pub summary_lsa: HashMap<LsaIdentifer, summary::SummaryLSA>,
    pub as_external_lsa: HashMap<LsaIdentifer, as_external::ASExternalLSA>,
}

impl LsaDb {
    pub fn empty() -> Self {
        Self {
            router_lsa: HashMap::new(),
            network_lsa: HashMap::new(),
            summary_lsa: HashMap::new(),
            as_external_lsa: HashMap::new(),
        }
    }
}

pub enum Lsa {
    Router(router::RouterLSA),
    Network(network::NetworkLSA),
    Summary(summary::SummaryLSA),
    ASExternal(as_external::ASExternalLSA),
}

impl Lsa {
    pub fn to_be_bytes(&self) -> Vec<u8> {
        match self {
            Lsa::Router(lsa) => lsa.to_be_bytes(),
            Lsa::Network(lsa) => lsa.to_be_bytes(),
            Lsa::Summary(lsa) => lsa.to_be_bytes(),
            Lsa::ASExternal(lsa) => lsa.to_be_bytes(),
        }
    }
    pub fn length(&self) -> usize {
        match self {
            Lsa::Router(lsa) => lsa.length(),
            Lsa::Network(lsa) => lsa.length(),
            Lsa::Summary(lsa) => lsa.length(),
            Lsa::ASExternal(lsa) => lsa.length(),
        }
    }
}
