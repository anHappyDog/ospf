use core::net;
use std::str::FromStr;

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
    pub fn length(&self) -> usize {
        20
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

pub trait LinkStateAdvertisement {
    fn to_bytes(&self) -> Vec<u8>;
    fn length(&self) -> usize;
}
