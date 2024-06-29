use core::net;

use crate::area::lsdb;

#[derive(Clone)]
pub struct NetworkLSA {
    pub header: super::Header,
    pub mask: u32,
    pub attached_rtrs: Vec<u32>,
}

pub const NETWORK_LSA_TYPE: u8 = 2;

impl NetworkLSA {
    pub async fn new(_iaddr: net::Ipv4Addr) -> Self {
        unimplemented!()
    }
    pub fn build_identifier(&self) -> lsdb::LsaIdentifer {
        lsdb::LsaIdentifer {
            lsa_type: NETWORK_LSA_TYPE as u32,
            link_state_id: self.header.link_state_id,
            advertising_router: self.header.advertising_router,
        }
    }
    pub fn length(&self) -> usize {
        super::Header::length() + 4 + self.attached_rtrs.len() * 4
    }
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.to_be_bytes());
        bytes.extend_from_slice(&self.mask.to_be_bytes());
        for rtr in &self.attached_rtrs {
            bytes.extend_from_slice(&rtr.to_be_bytes());
        }
        bytes
    }
    pub fn try_from_be_bytes(payload: &[u8]) -> Option<Self> {
        if payload.len() < super::Header::length() + 4 {
            return None;
        }
        let header = super::Header::try_from_be_bytes(&payload[..super::Header::length()])?;
        let mask = u32::from_be_bytes([
            payload[super::Header::length()],
            payload[super::Header::length() + 1],
            payload[super::Header::length() + 2],
            payload[super::Header::length() + 3],
        ]);
        let mut attached_rtrs = Vec::new();
        let mut offset = super::Header::length() + 4;
        while offset < payload.len() {
            let rtr = u32::from_be_bytes([
                payload[offset],
                payload[offset + 1],
                payload[offset + 2],
                payload[offset + 3],
            ]);
            attached_rtrs.push(rtr);
            offset += 4;
        }
        Some(Self {
            header,
            mask,
            attached_rtrs,
        })
    }
}
