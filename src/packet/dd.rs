use core::net;
use std::net::Ipv4Addr;

use pnet::{
    packet::{
        ip::IpNextHeaderProtocol,
        ipv4::{self, Ipv4Packet, MutableIpv4Packet},
    },
    util::Octets,
};

use crate::{lsa, neighbor, OSPF_IP_PROTOCOL};

pub const DD_TYPE: u8 = 2;
#[derive(Clone)]
pub struct DD {
    pub header: super::OspfHeader,
    pub interface_mtu: u16,
    pub options: u8,
    pub flags: u8,
    pub dd_sequence_number: u32,
    pub lsa_headers: Vec<crate::lsa::Header>,
}

pub const FLAG_R_BIT: u8 = 0b0000_1000;
pub const FLAG_I_BIT: u8 = 0b0000_0100;
pub const FLAG_M_BIT: u8 = 0b0000_0010;
pub const FLAG_MS_BIT: u8 = 0b0000_0001;

impl DD {
    pub fn empty() -> Self {
        Self {
            header: super::OspfHeader::empty(),
            interface_mtu: 0,
            options: 0,
            flags: 0,
            dd_sequence_number: 0,
            lsa_headers: Vec::new(),
        }
    }

    pub fn has_option_e(&self) -> bool {
        self.options & crate::OPTION_E == crate::OPTION_E
    }
    pub fn has_option_mc(&self) -> bool {
        self.options & crate::OPTION_MC == crate::OPTION_MC
    }
    pub fn has_option_np(&self) -> bool {
        self.options & crate::OPTION_NP == crate::OPTION_NP
    }
    pub fn has_option_dc(&self) -> bool {
        self.options & crate::OPTION_DC == crate::OPTION_DC
    }

    pub fn is_flag_i_set(&self) -> bool {
        self.flags & FLAG_I_BIT == FLAG_I_BIT
    }
    pub fn is_flag_m_set(&self) -> bool {
        self.flags & FLAG_M_BIT == FLAG_M_BIT
    }
    pub fn is_flag_ms_set(&self) -> bool {
        self.flags & FLAG_MS_BIT == FLAG_MS_BIT
    }

    pub fn set_flag_i(&mut self) {
        self.flags |= FLAG_I_BIT;
    }
    pub fn set_flag_m(&mut self) {
        self.flags |= FLAG_M_BIT;
    }
    pub fn set_flag_ms(&mut self) {
        self.flags |= FLAG_MS_BIT;
    }

    pub fn build_ipv4_packet<'a>(
        &'a self,
        buffer: &'a mut Vec<u8>,
        source_addr: Ipv4Addr,
        destination_addr: Ipv4Addr,
    ) -> Option<Ipv4Packet> {
        match MutableIpv4Packet::new(buffer.as_mut_slice()) {
            Some(mut packet) => {
                println!("DD packet length is {}", self.length() as u16);
                packet.set_source(source_addr);
                packet.set_destination(destination_addr);
                packet.set_header_length(5);
                packet.set_total_length(20 + self.length() as u16);

                packet.set_payload(&self.to_be_bytes());
                packet.set_version(4);

                packet.set_next_level_protocol(IpNextHeaderProtocol(OSPF_IP_PROTOCOL));
                packet.set_ttl(1);
                println!("DD packet length is {}", self.length() as u16);

                packet.set_checksum(ipv4::checksum(&packet.to_immutable()));
                Some(packet.consume_to_immutable())
            }
            None => {
                return None;
            }
        }
    }
    pub fn try_from_be_bytes(payload: &[u8]) -> Option<Self> {
        unimplemented!()
    }
    pub fn length(&self) -> usize {
        super::OspfHeader::length() + 8 + self.lsa_headers.len() * lsa::Header::length()
    }
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.to_be_bytes());
        bytes.extend_from_slice(&self.interface_mtu.to_be_bytes());
        bytes.extend_from_slice(&self.options.octets());
        bytes.extend_from_slice(&self.flags.octets());
        bytes.extend_from_slice(&self.dd_sequence_number.to_be_bytes());
        self.lsa_headers.iter().for_each(|t| {
            bytes.extend_from_slice(&t.to_be_bytes());
        });
        bytes
    }
    pub async fn received(
        dd_packet: DD,
        packet_source_addr: net::Ipv4Addr,
        int_ipv4_addr: net::Ipv4Addr,
    ) {
  
    }
}
