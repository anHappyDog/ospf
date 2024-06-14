use super::{LinkStateAdvertisement, LinkStateAdvertisementHeader};
use std::net;

pub const AS_EXTERNAL_LINK_STATE_TYPE: u8 = 5;

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
        length += LinkStateAdvertisementHeader::length();
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

    fn to_be_bytes(&self) -> Vec<u8> {
        let mut result = vec![];
        result.extend(self.header.to_be_bytes());
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

    pub fn from_be_bytes(data: &[u8]) -> Self {
        let header = LinkStateAdvertisementHeader::from_be_bytes(&data[0..20]);
        let network_mask = net::Ipv4Addr::new(data[20], data[21], data[22], data[23]);
        let feature = data[24];
        let metric = [data[25], data[26], data[27]];
        let forwarding_addr = net::Ipv4Addr::new(data[28], data[29], data[30], data[31]);
        let external_route_tag = u32::from_be_bytes([data[32], data[33], data[34], data[35]]);
        let tos_feature = data[36];
        let tos_metric = [data[37], data[38], data[39]];
        let tos_forwarding_addr = net::Ipv4Addr::new(data[40], data[41], data[42], data[43]);
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
