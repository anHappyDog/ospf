use super::{LinkStateAdvertisement, LinkStateAdvertisementHeader};
use std::net;

pub const NETWORK_LINK_STATE_TYPE: u8 = 2;

pub struct NetworkLinkStateAdvertisement {
    pub header: LinkStateAdvertisementHeader,
    pub network_mask: u32,
    pub attached_routers: Vec<net::Ipv4Addr>,
}

unsafe impl Send for NetworkLinkStateAdvertisement {}

impl LinkStateAdvertisement for NetworkLinkStateAdvertisement {
    fn length(&self) -> usize {
        let mut length = 0;
        length += LinkStateAdvertisementHeader::length();
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
    fn to_be_bytes(&self) -> Vec<u8> {
        let mut result = vec![];
        result.extend(self.header.to_be_bytes());
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
    pub fn from_be_bytes(data: &[u8]) -> Self {
        let header = LinkStateAdvertisementHeader::from_be_bytes(&data[0..20]);
        let network_mask = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        let mut attached_routers = vec![];
        let mut index = 24;
        while index < data.len() {
            attached_routers.push(net::Ipv4Addr::new(
                data[index],
                data[index + 1],
                data[index + 2],
                data[index + 3],
            ));
            index += 4;
        }
        Self {
            header,
            network_mask,
            attached_routers,
        }
    }
}
