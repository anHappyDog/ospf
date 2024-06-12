use super::{LinkStateAdvertisement, LinkStateAdvertisementHeader};
use std::net;

pub struct NetworkLinkStateAdvertisement {
    pub header: LinkStateAdvertisementHeader,
    pub network_mask: u32,
    pub attached_routers: Vec<net::Ipv4Addr>,
}

impl LinkStateAdvertisement for NetworkLinkStateAdvertisement {
    fn length(&self) -> usize {
        let mut length = 0;
        length += self.header.length();
        length += 4;
        length += 4 * self.attached_routers.len();
        length
    }
    fn to_bytes(&self) -> Vec<u8> {
        let mut result = vec![];
        result.extend(self.header.to_bytes());
        result.extend(self.network_mask.to_be_bytes());
        for router in &self.attached_routers {
            result.extend(router.octets().iter());
        }
        result
    }
}

impl NetworkLinkStateAdvertisement {
    pub fn new(
        header: LinkStateAdvertisementHeader,
        network_mask: u32,
        attached_routers: Vec<net::Ipv4Addr>,
    ) -> Self {
        Self {
            header,
            network_mask,
            attached_routers,
        }
    }
}
