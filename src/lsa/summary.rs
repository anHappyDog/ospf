use super::{LinkStateAdvertisement, LinkStateAdvertisementHeader};

pub struct SummaryLinkStateAdvertisement {
    pub header: LinkStateAdvertisementHeader,
    pub network_mask: u32,
    pub attached_routers: Vec<u32>,
}

impl LinkStateAdvertisement for SummaryLinkStateAdvertisement {}

impl SummaryLinkStateAdvertisement {
    
}