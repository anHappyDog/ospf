use super::{LinkStateAdvertisement, LinkStateAdvertisementHeader};

pub const ROUTER_LINK_STATE_TYPE: u8  = 1;  

pub struct RouterLink {
    pub link_id: u32,
    pub link_data: u32,
    pub link_type: u8,
    pub link_tos: u8,
    pub link_metric: u16,
}

/// # struct RouterLinkStateAdvertisement
/// doc to be implemented
pub struct RouterLinkStateAdvertisement {
    pub header: LinkStateAdvertisementHeader,
    pub feature: u16,
    pub link_count: u16,
    pub links: Vec<Box<RouterLink>>,
}

unsafe impl Send for RouterLinkStateAdvertisement {}

impl LinkStateAdvertisement for RouterLinkStateAdvertisement {
    /// # wrong
    fn length(&self) -> usize {
        let mut length = 0;
        length += LinkStateAdvertisementHeader::length();
        length += 4;
        length += 4 * self.link_count as usize;
        length
    }
    fn to_be_bytes(&self) -> Vec<u8> {
        let mut result = vec![];
        result.extend(self.header.to_be_bytes());
        result.extend(self.feature.to_be_bytes());
        result.extend(self.link_count.to_be_bytes());
        for link in &self.links {
            result.extend(link.link_id.to_be_bytes());
            result.extend(link.link_data.to_be_bytes());
            result.push(link.link_type);
            result.push(link.link_tos);
            result.extend(link.link_metric.to_be_bytes());
        }
        result
    }
    fn to_bytes(&self) -> Vec<u8> {
        let mut result = vec![];
        result.extend(self.header.to_bytes());
        result.extend(self.feature.to_be_bytes());
        result.extend(self.link_count.to_be_bytes());
        for link in &self.links {
            result.extend(link.link_id.to_be_bytes());
            result.extend(link.link_data.to_be_bytes());
            result.push(link.link_type);
            result.push(link.link_tos);
            result.extend(link.link_metric.to_be_bytes());
        }
        result
    }
}

impl RouterLinkStateAdvertisement {
    pub fn from_be_bytes(data: &[u8]) -> Self {
        let header = LinkStateAdvertisementHeader::from_be_bytes(&data[0..20]);
        let feature = u16::from_be_bytes([data[20], data[21]]);
        let link_count = u16::from_be_bytes([data[22], data[23]]);
        let mut links = vec![];
        let mut index = 24;
        for _ in 0..link_count {
            links.push(Box::new(RouterLink {
                link_id: u32::from_be_bytes([
                    data[index],
                    data[index + 1],
                    data[index + 2],
                    data[index + 3],
                ]),
                link_data: u32::from_be_bytes([
                    data[index + 4],
                    data[index + 5],
                    data[index + 6],
                    data[index + 7],
                ]),
                link_type: data[index + 8],
                link_tos: data[index + 9],
                link_metric: u16::from_be_bytes([data[index + 10], data[index + 11]]),
            }));
            index += 12;
        }
        Self {
            header,
            feature,
            link_count,
            links,
        }
    }
    pub fn new(
        header: LinkStateAdvertisementHeader,
        feature: u16,
        link_count: u16,
        links: Vec<Box<RouterLink>>,
    ) -> Self {
        Self {
            header,
            feature,
            link_count,
            links,
        }
    }
}
