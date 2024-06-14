use super::{LinkStateAdvertisement, LinkStateAdvertisementHeader};

pub const SUMMARY_LINK_STATE_TYPE_3: u8 = 3;
pub const SUMMARY_LINK_STATE_TYPE_4: u8 = 4;

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
        length += LinkStateAdvertisementHeader::length();
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
    fn to_be_bytes(&self) -> Vec<u8> {
        let mut result = vec![];
        result.extend(self.header.to_be_bytes());
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
    pub fn from_be_bytes(data: &[u8]) -> Self {
        let header = LinkStateAdvertisementHeader::from_be_bytes(&data[0..20]);
        let network_mask = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        let zero_pad = data[24];
        let metric = [data[25], data[26], data[27]];
        let tos = data[28];
        let tos_metric = [data[29], data[30], data[31]];
        let tos_additional_info = if data.len() > 32 {
            Some(u32::from_be_bytes([data[32], data[33], data[34], data[35]]))
        } else {
            None
        };
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
