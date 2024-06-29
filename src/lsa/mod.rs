use core::net;

use as_external::AS_EXTERNAL_LSA_TYPE;
use network::NETWORK_LSA_TYPE;
use pnet::util::Octets;
use router::ROUTER_LSA_TYPE;
use summary::{SUMMARY_LSA_TYPE_3, SUMMARY_LSA_TYPE_4};

use crate::area::lsdb;

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

#[derive(Clone)]
pub enum Lsa {
    Router(router::RouterLSA),
    Network(network::NetworkLSA),
    Summary(summary::SummaryLSA),
    ASExternal(as_external::ASExternalLSA),
}

impl Lsa {
    pub fn copy_header(&self) -> Header {
        match self {
            Lsa::Router(lsa) => lsa.header,
            Lsa::Network(lsa) => lsa.header,
            Lsa::Summary(lsa) => lsa.header,
            Lsa::ASExternal(lsa) => lsa.header,
        }
    }
    pub fn build_identifier(&self) -> lsdb::LsaIdentifer {
        match self {
            Lsa::Router(lsa) => lsa.build_identifier(),
            Lsa::Network(lsa) => lsa.build_identifier(),
            Lsa::Summary(lsa) => lsa.build_identifier(),
            Lsa::ASExternal(lsa) => lsa.build_identifier(),
        }
    }
    pub fn try_from_be_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Header::length() {
            return None;
        }
        let header = Header::try_from_be_bytes(&bytes[..Header::length()])?;
        match header.lsa_type {
            ROUTER_LSA_TYPE => router::RouterLSA::try_from_be_bytes(bytes).map(Lsa::Router),
            NETWORK_LSA_TYPE => network::NetworkLSA::try_from_be_bytes(bytes).map(Lsa::Network),
            SUMMARY_LSA_TYPE_3 => summary::SummaryLSA::try_from_be_bytes(bytes).map(Lsa::Summary),
            SUMMARY_LSA_TYPE_4 => summary::SummaryLSA::try_from_be_bytes(bytes).map(Lsa::Summary),
            AS_EXTERNAL_LSA_TYPE => {
                as_external::ASExternalLSA::try_from_be_bytes(bytes).map(Lsa::ASExternal)
            }
            _ => None,
        }
    }
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

pub fn calculate_fletcher16(data: &[u8]) -> u16 {
    let mut sum1: u16 = 0;
    let mut sum2: u16 = 0;

    for &byte in data {
        sum1 = (sum1 + byte as u16) % 255;
        sum2 = (sum2 + sum1) % 255;
    }

    (sum2 << 8) | sum1
}

pub fn calculate_lsa_checksum(lsa: &mut [u8]) -> u16 {
    let checksum_offset = 16;

    lsa[checksum_offset] = 0;
    lsa[checksum_offset + 1] = 0;

    let checksum = calculate_fletcher16(lsa);

    checksum
}
