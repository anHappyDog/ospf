use crate::area::lsdb;

use super::Header;

#[derive(Clone)]
pub struct ASExternalLSA {
    pub header: Header,
    pub network_mask: u32,
    pub e_bits: u8,
    pub metric: [u8; 3],
    pub forwarding_address: u32,
    pub external_route_tag: u32,
    pub tos: Vec<TosDes>,
}

pub const AS_EXTERNAL_LSA_TYPE: u8 = 5;

#[derive(Clone, Copy)]
pub struct TosDes {
    pub tos_ebits: u8,
    pub tos_metric: [u8; 3],
    pub tos_forwarding_address: u32,
    pub tos_external_route_tag: u32,
}

impl TosDes {
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.tos_ebits);
        bytes.extend_from_slice(&self.tos_metric);
        bytes.extend_from_slice(&self.tos_forwarding_address.to_be_bytes());
        bytes.extend_from_slice(&self.tos_external_route_tag.to_be_bytes());
        bytes
    }

    pub fn length(&self) -> usize {
        12
    }
    pub fn try_from_be_bytes(payload: &[u8]) -> Option<Self> {
        if payload.len() < 12 {
            return None;
        }
        Some(Self {
            tos_ebits: payload[0],
            tos_metric: [payload[1], payload[2], payload[3]],
            tos_forwarding_address: u32::from_be_bytes([
                payload[4], payload[5], payload[6], payload[7],
            ]),
            tos_external_route_tag: u32::from_be_bytes([
                payload[8],
                payload[9],
                payload[10],
                payload[11],
            ]),
        })
    }
}

impl ASExternalLSA {
    pub fn build_identifier(&self) -> lsdb::LsaIdentifer {
        lsdb::LsaIdentifer {
            lsa_type: self.header.lsa_type as u32,
            link_state_id: self.header.link_state_id,
            advertising_router: self.header.advertising_router,
        }
    }
    pub fn try_from_be_bytes(payload: &[u8]) -> Option<Self> {
        if payload.len() < Header::length() + 13 {
            return None;
        }
        let header_length = Header::length();
        let header = Header::try_from_be_bytes(&payload[..header_length])?;
        let network_mask = u32::from_be_bytes([
            payload[header_length],
            payload[header_length + 1],
            payload[header_length + 2],
            payload[header_length + 3],
        ]);
        let e_bits = payload[header_length + 4];
        let metric = [
            payload[header_length + 5],
            payload[header_length + 6],
            payload[header_length + 7],
        ];
        let forwarding_address = u32::from_be_bytes([
            payload[header_length + 8],
            payload[header_length + 9],
            payload[header_length + 10],
            payload[header_length + 11],
        ]);
        let external_route_tag = u32::from_be_bytes([
            payload[header_length + 12],
            payload[header_length + 13],
            payload[header_length + 14],
            payload[header_length + 15],
        ]);
        let mut tos = Vec::new();
        let mut offset = header_length + 16;
        while offset + 12 <= payload.len() {
            let tos_value = TosDes::try_from_be_bytes(&payload[offset..offset + 12])?;
            tos.push(tos_value);
            offset += 12;
        }
        Some(Self {
            header,
            network_mask,
            e_bits,
            metric,
            forwarding_address,
            external_route_tag,
            tos,
        })
    }

    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.to_be_bytes());
        bytes.extend_from_slice(&self.network_mask.to_be_bytes());
        bytes.push(self.e_bits);
        bytes.extend_from_slice(&self.metric);
        bytes.extend_from_slice(&self.forwarding_address.to_be_bytes());
        bytes.extend_from_slice(&self.external_route_tag.to_be_bytes());
        for tos in &self.tos {
            bytes.extend_from_slice(&tos.to_be_bytes());
        }
        bytes
    }
    pub fn length(&self) -> usize {
        let mut length = Header::length() + 13;
        for tos in &self.tos {
            length += tos.length();
        }
        length
    }
}
