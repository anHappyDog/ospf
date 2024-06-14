use crate::lsa::LinkStateAdvertisementHeader;

use super::{OspfPacket, OspfPacketHeader};

pub const FEATURE_BIT_I: u8 = 1 << 2;
pub const FEATURE_BIT_M: u8 = 1 << 1;
pub const FEATURE_BIT_MS: u8 = 1 << 0;

pub const DATA_DESCRIPTION_PACKET_TYPE: u8 = 2;

pub struct DataDescriptionPacket {
    pub header: OspfPacketHeader,
    pub interface_mtu: u16,
    pub options: u8,
    pub features: u8,
    pub dd_sequence_number: u32,
    pub lsa_headers: Vec<LinkStateAdvertisementHeader>,
}

impl OspfPacket for DataDescriptionPacket {
    fn length(&self) -> usize {
        let mut length = 0;
        length += OspfPacketHeader::length();
        length += std::mem::size_of::<u16>();
        length += std::mem::size_of::<u8>();
        length += std::mem::size_of::<u8>();
        length += std::mem::size_of::<u32>();
        length += self.lsa_headers.len() * LinkStateAdvertisementHeader::length();
        length
    }
    fn calculate_checksum(&mut self) {}

    fn to_bytes(&self) -> Vec<u8> {
        let mut result = vec![];
        result.extend(self.header.to_bytes());
        result.extend(self.interface_mtu.to_be_bytes());
        result.push(self.options);
        result.push(self.features);
        result.extend(self.dd_sequence_number.to_be_bytes());
        for lsa_header in &self.lsa_headers {
            result.extend(lsa_header.to_bytes());
        }
        result
    }
    fn to_be_bytes(&self) -> Vec<u8> {
        let mut result = vec![];
        result.extend(self.header.to_be_bytes());
        result.extend(self.interface_mtu.to_be_bytes());
        result.push(self.options);
        result.push(self.features);
        result.extend(self.dd_sequence_number.to_be_bytes());
        for lsa_header in &self.lsa_headers {
            result.extend(lsa_header.to_be_bytes());
        }
        result
    }
    fn get_type(&self) -> u8 {
        DATA_DESCRIPTION_PACKET_TYPE
    }
}

impl DataDescriptionPacket {
    pub fn new(
        header: OspfPacketHeader,
        interface_mtu: u16,
        options: u8,
        features: u8,
        dd_sequence_number: u32,
        lsa_headers: Vec<LinkStateAdvertisementHeader>,
    ) -> Self {
        Self {
            header,
            interface_mtu,
            options,
            features,
            dd_sequence_number,
            lsa_headers,
        }
    }
    pub fn from_be_bytes(data: &[u8]) -> Self {
        let header = OspfPacketHeader::from_be_bytes(&data[0..24]);
        let interface_mtu = u16::from_be_bytes([data[24], data[25]]);
        let options = data[26];
        let features = data[27];
        let dd_sequence_number = u32::from_be_bytes([data[28], data[29], data[30], data[31]]);
        let mut lsa_headers = vec![];
        let mut index = 32;
        while index < data.len() {
            lsa_headers.push(LinkStateAdvertisementHeader::from_be_bytes(
                &data[index..index + 20],
            ));
            index += 20;
        }
        Self {
            header,
            interface_mtu,
            options,
            features,
            dd_sequence_number,
            lsa_headers,
        }
    }
}
