use super::{LinkStateAdvertisement, LinkStateAdvertisementHeader};
use std::net;

pub struct AsExternalLinkStateAdvertisement {
    pub header: LinkStateAdvertisementHeader,
    pub network_mask: net::Ipv4Addr,
    pub feature: u8,
    pub metric: [u8; 3],
    pub forwarding_addr: net::Ipv4Addr,
    pub external_route_tag: u32,
    pub tos_feature: u8,
    pub tos_metric: [u8; 3],
    pub tos_forwarding_addr: net::Ipv4Addr,
}

impl LinkStateAdvertisement for AsExternalLinkStateAdvertisement {
    fn length(&self) -> usize {
        let mut length = 0;
        length += self.header.length();
        length += 4;
        length += 1;
        length += 3;
        length += 4;
        length += 4;
        length += 1;
        length += 3;
        length += 4;
        length
    }
    fn to_bytes(&self) -> Vec<u8> {
        let mut result = vec![];
        result.extend(self.header.to_bytes());
        result.extend(self.network_mask.octets().iter());
        result.push(self.feature);
        result.extend(&self.metric);
        result.extend(self.forwarding_addr.octets().iter());
        result.extend(self.external_route_tag.to_be_bytes());
        result.push(self.tos_feature);
        result.extend(&self.tos_metric);
        result.extend(self.tos_forwarding_addr.octets().iter());
        result
    }
}

impl AsExternalLinkStateAdvertisement {
    pub fn new(
        header: LinkStateAdvertisementHeader,
        network_mask: net::Ipv4Addr,
        feature: u8,
        metric: [u8; 3],
        forwarding_addr: net::Ipv4Addr,
        external_route_tag: u32,
        tos_feature: u8,
        tos_metric: [u8; 3],
        tos_forwarding_addr: net::Ipv4Addr,
    ) -> Self {
        Self {
            header,
            network_mask,
            feature,
            metric,
            forwarding_addr,
            external_route_tag,
            tos_feature,
            tos_metric,
            tos_forwarding_addr,
        }
    }
}
