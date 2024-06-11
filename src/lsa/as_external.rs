use super::{LinkStateAdvertisement, LinkStateAdvertisementHeader};

pub struct AsExternalLinkStateAdvertisement {
    pub header: LinkStateAdvertisementHeader,
    pub metric: u32,
    pub forwarding_address: u32,
    pub external_route_tag: u32,
}

impl LinkStateAdvertisement for AsExternalLinkStateAdvertisement {}


impl AsExternalLinkStateAdvertisement {
    
}