use super::{LinkStateAdvertisement, LinkStateAdvertisementHeader};

/// please read the doc agian and change it.
pub struct SummaryLinkStateAdvertisement {
    pub header: LinkStateAdvertisementHeader,
    pub network_mask: u32,
    pub zero_pad: u8,
    pub metric: [u8; 3],
    pub tos: u8,
    pub tos_metric: [u8; 3],
    pub tos_additional_info: Option<u32>,
}

impl LinkStateAdvertisement for SummaryLinkStateAdvertisement {
    fn length(&self) -> usize {
        let mut length = 0;
        length += self.header.length();
        length += 4;
        length += 1;
        length += 3;
        length += 1;
        length += 3;
        if let Some(_) = self.tos_additional_info {
            length += 4;
        }
        length
    }
    fn to_bytes(&self) -> Vec<u8> {
        let mut result = vec![];
        result.extend(self.header.to_bytes());
        result.extend(self.network_mask.to_be_bytes());
        result.push(self.zero_pad);
        result.extend(&self.metric);
        result.push(self.tos);
        result.extend(&self.tos_metric);
        if let Some(tos_additional_info) = self.tos_additional_info {
            result.extend(tos_additional_info.to_be_bytes());
        }
        result
    }
}

impl SummaryLinkStateAdvertisement {
    pub fn new(
        header: LinkStateAdvertisementHeader,
        network_mask: u32,
        zero_pad: u8,
        metric: [u8; 3],
        tos: u8,
        tos_metric: [u8; 3],
        tos_additional_info: Option<u32>,
    ) -> Self {
        Self {
            header,
            network_mask,
            zero_pad,
            metric,
            tos,
            tos_metric,
            tos_additional_info,
        }
    }
}
