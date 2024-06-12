use super::{LinkStateAdvertisement, LinkStateAdvertisementHeader};

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

impl LinkStateAdvertisement for RouterLinkStateAdvertisement {
    /// # wrong
    fn length(&self) -> usize {
        let mut length = 0;
        length += self.header.length();
        length += 4;
        length += 4 * self.link_count as usize;
        length
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
