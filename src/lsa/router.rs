use super::{LinkStateAdvertisement, LinkStateAdvertisementHeader};


pub struct RouterLinkStateAdvertisement {
    pub header: LinkStateAdvertisementHeader,
    pub links: Vec<RouterLink>,
}


impl LinkStateAdvertisement for RouterLinkStateAdvertisement {}


impl RouterLinkStateAdvertisement {
    
}