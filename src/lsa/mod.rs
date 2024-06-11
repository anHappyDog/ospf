pub mod as_external;
pub mod network;
pub mod router;
pub mod summary;

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

pub trait LinkStateAdvertisement {}
