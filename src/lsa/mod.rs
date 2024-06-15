use core::net;
use std::str::FromStr;

use as_external::AS_EXTERNAL_LINK_STATE_TYPE;
use network::NETWORK_LINK_STATE_TYPE;
use router::ROUTER_LINK_STATE_TYPE;
use summary::{SUMMARY_LINK_STATE_TYPE_3, SUMMARY_LINK_STATE_TYPE_4};

pub mod as_external;
pub mod network;
pub mod router;
pub mod summary;

#[allow(non_upper_case_globals)]
pub const LSRefreshTime: u32 = 1800;

#[allow(non_upper_case_globals)]
pub const MinLSInterval: u32 = 5;

#[allow(non_upper_case_globals)]
pub const MaxAge: u32 = 1;

#[allow(non_upper_case_globals)]
pub const CheckAge: u32 = 3600;
#[allow(non_upper_case_globals)]
pub const MaxAgeDiff: u32 = 900;
#[allow(non_upper_case_globals)]
pub const LSInfinity: u32 = 0xffffff;
#[allow(non_upper_case_globals)]
pub const DefaultDesination: net::Ipv4Addr = net::Ipv4Addr::from_bits(0);
#[allow(non_upper_case_globals)]
pub const InitialSequenceNumber: u32 = 0x7fffffff;

pub struct LinkStateAdvertisementHeader {
    pub age: u16,
    pub options: u8,
    pub lsa_type: u8,
    pub link_state_id: u32,
    pub advertising_router: u32,
    pub sequence_number: u32,
    pub checksum: u16,
    pub length: u16,
}

unsafe impl Send for LinkStateAdvertisementHeader {}

impl LinkStateAdvertisementHeader {
    pub fn new(
        age: u16,
        options: u8,
        lsa_type: u8,
        link_state_id: u32,
        advertising_router: u32,
        sequence_number: u32,
        checksum: u16,
        length: u16,
    ) -> Self {
        Self {
            age,
            options,
            lsa_type,
            link_state_id,
            advertising_router,
            sequence_number,
            checksum,
            length,
        }
    }
    pub fn length() -> usize {
        20
    }
    pub fn from_be_bytes(bytes: &[u8]) -> Self {
        Self {
            age: u16::from_be_bytes([bytes[0], bytes[1]]),
            options: bytes[2],
            lsa_type: bytes[3],
            link_state_id: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            advertising_router: u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            sequence_number: u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            checksum: u16::from_be_bytes([bytes[16], bytes[17]]),
            length: u16::from_be_bytes([bytes[18], bytes[19]]),
        }
    }
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(20);
        bytes.extend_from_slice(&self.age.to_be_bytes());
        bytes.push(self.options);
        bytes.push(self.lsa_type);
        bytes.extend_from_slice(&self.link_state_id.to_be_bytes());
        bytes.extend_from_slice(&self.advertising_router.to_be_bytes());
        bytes.extend_from_slice(&self.sequence_number.to_be_bytes());
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.length.to_be_bytes());
        bytes
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(20);
        bytes.extend_from_slice(&self.age.to_be_bytes());
        bytes.push(self.options);
        bytes.push(self.lsa_type);
        bytes.extend_from_slice(&self.link_state_id.to_be_bytes());
        bytes.extend_from_slice(&self.advertising_router.to_be_bytes());
        bytes.extend_from_slice(&self.sequence_number.to_be_bytes());
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.length.to_be_bytes());
        bytes
    }
}

pub fn get_lsa_from_be_bytes(data: &[u8]) -> Box<dyn LinkStateAdvertisement> {
    let header = LinkStateAdvertisementHeader::from_be_bytes(&data[0..20]);
    match header.lsa_type {
        ROUTER_LINK_STATE_TYPE => {
            Box::new(router::RouterLinkStateAdvertisement::from_be_bytes(data))
        }
        NETWORK_LINK_STATE_TYPE => {
            Box::new(network::NetworkLinkStateAdvertisement::from_be_bytes(data))
        }
        SUMMARY_LINK_STATE_TYPE_3 => {
            Box::new(summary::SummaryLinkStateAdvertisement::from_be_bytes(data))
        }
        SUMMARY_LINK_STATE_TYPE_4 => {
            Box::new(summary::SummaryLinkStateAdvertisement::from_be_bytes(data))
        }
        AS_EXTERNAL_LINK_STATE_TYPE => {
            Box::new(as_external::AsExternalLinkStateAdvertisement::from_be_bytes(data))
        }
        _ => panic!("Unknown LSA type"),
    }
}

pub trait LinkStateAdvertisement {
    fn to_be_bytes(&self) -> Vec<u8>;
    fn to_bytes(&self) -> Vec<u8>;
    fn length(&self) -> usize;
}
