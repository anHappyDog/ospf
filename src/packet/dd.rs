use core::net;
use std::{net::Ipv4Addr, thread::panicking};

use pnet::{
    packet::{
        ip::IpNextHeaderProtocol,
        ipv4::{self, Ipv4Packet, MutableIpv4Packet},
    },
    util::Octets,
};

use crate::{
    interface::{self, NetworkType},
    lsa, neighbor, OSPF_IP_PROTOCOL,
};

use super::ospf_packet_checksum;

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
    pub async fn new(
        iaddr: net::Ipv4Addr,
        _naddr: net::Ipv4Addr,
        dd_options: u8,
        dd_flags: u8,
        seqno: u32,
    ) -> Self {
        let mut packet = Self::empty();
        let interfaces_map = interface::INTERFACE_MAP.read().await;
        let interface = interfaces_map.get(&iaddr).unwrap();
        let area_id = interface.area_id;
        let auth_type = interface.auth_type;
        let auth_key = interface.auth_key;
        let network_type = interface.network_type;
        drop(interfaces_map);

        packet.header.router_id = crate::ROUTER_ID.clone().into();
        packet.header.area_id = area_id.into();
        packet.header.packet_type = DD_TYPE;
        packet.header.packet_length = 0;
        packet.header.checksum = 0;
        packet.header.auth_type = auth_type;
        packet.header.authentication = auth_key;
        packet.interface_mtu = match network_type {
            NetworkType::VirtualLink => 0,
            _ => crate::IPV4_PACKET_MTU as u16,
        };
        packet.options = dd_options;
        packet.flags = dd_flags;
        packet.dd_sequence_number = seqno;
        // TODO fill the lsa_headers,but remember can not exceed the mtu
        
        packet.header.packet_length = packet.length() as u16;
        packet.header.checksum = ospf_packet_checksum(&packet.to_be_bytes());
        packet
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
    pub async fn received(packet: DD, naddr: net::Ipv4Addr, iaddr: net::Ipv4Addr) {
        let mut old_status = neighbor::get_status(iaddr, naddr).await;
        loop {
            match old_status {
                neighbor::status::Status::Down => {
                    crate::util::error(&format!(
                        "Received DD packet from neighbor {} in Down state,rejected.",
                        naddr
                    ));
                    return;
                }
                neighbor::status::Status::Attempt => {
                    crate::util::error(&format!(
                        "Received DD packet from neighbor {} in Attempt state,rejected.",
                        naddr
                    ));
                    return;
                }
                neighbor::status::Status::Init => {
                    crate::util::error(&format!(
                        "Received DD packet from neighbor {} in Init state",
                        naddr
                    ));
                    neighbor::event::Event::two_way_received(naddr, iaddr).await;
                    old_status = neighbor::get_status(iaddr, naddr).await;
                    if neighbor::status::Status::ExStart == old_status {
                        continue;
                    }
                    return;
                }
                neighbor::status::Status::TwoWay => {
                    crate::util::debug(&format!(
                        "Received DD packet from neighbor {} in TwoWay state,ignored.",
                        naddr
                    ));
                    return;
                }
                neighbor::status::Status::ExStart => {
                    crate::util::debug(&format!(
                        "Received DD packet from neighbor {} in ExStart state",
                        naddr
                    ));
                    let cur_dd_seq = neighbor::get_ddseqno(iaddr, naddr).await;
                    if packet.is_flag_i_set()
                        && packet.is_flag_m_set()
                        && packet.is_flag_ms_set()
                        && packet.lsa_headers.is_empty()
                        && packet.header.router_id > crate::ROUTER_ID.clone()
                    {
                        neighbor::set_option(iaddr, naddr, packet.options).await;
                        neighbor::set_ddseqno(iaddr, naddr, packet.dd_sequence_number).await;
                        neighbor::set_master(iaddr, naddr, true).await;
                    } else if !packet.is_flag_i_set()
                        && !packet.is_flag_m_set()
                        && packet.dd_sequence_number == cur_dd_seq
                        && packet.header.router_id < crate::ROUTER_ID.clone()
                    {
                        neighbor::set_option(iaddr, naddr, packet.options).await;
                        neighbor::set_master(iaddr, naddr, false).await;
                    } else {
                        crate::util::debug(&format!(
                            "Received DD packet from neighbor {} in ExStart state,but does not meet the two situations,ignored.",
                            naddr
                        ));
                        return;
                    }
                }
                neighbor::status::Status::Exchange => {
                    crate::util::debug(&format!(
                        "Received DD packet from neighbor {} in Exchange state",
                        naddr
                    ));
                    // check if the dd packet is duplicated
                    // let cur_dd_seq = neighbor::get_ddseqno(iaddr, naddr).await;
                    if neighbor::is_duplicated_dd(iaddr, naddr, &packet).await {
                        crate::util::debug(&format!(
                            "Received DD packet from neighbor {} in Exchange state,but is duplicated,ignored.",
                            naddr
                        ));
                        return;
                    }
                    let ddseqno = neighbor::get_ddseqno(iaddr, naddr).await;
                    let master = neighbor::is_master(iaddr, naddr).await;
                    if (packet.is_flag_ms_set()  && !master) || (!packet.is_flag_ms_set() && master) {
                        crate::util::error(&format!(
                            "Received DD packet from neighbor {} in Exchange state,but does not meet the two situations,ignored.",
                            naddr
                        ));
                        neighbor::event::send(iaddr, naddr, neighbor::event::Event::SeqNumberMismatch).await;
                        return;
                    }
                    if packet.is_flag_i_set() {
                        crate::util::error(&format!(
                            "Received DD packet from neighbor {} in Exchange state,ignored.",
                            naddr
                        ));
                        neighbor::event::send(iaddr, naddr, neighbor::event::Event::SeqNumberMismatch).await;
                        return ;
                    }
                    let prev_options = neighbor::get_options(iaddr, naddr).await;
                    if packet.options != prev_options {
                        crate::util::error(&format!(
                            "Received DD packet from neighbor {} in Exchange state,but options wrong,discarded.",
                            naddr
                        ));
                        neighbor::event::send(iaddr, naddr, neighbor::event::Event::SeqNumberMismatch).await;
                        return ;
                    }
                    if (master && packet.dd_sequence_number != ddseqno) || (!master && packet.dd_sequence_number + 1 != ddseqno) {
                        crate::util::error(&format!(
                            "Received DD packet from neighbor {} in Exchange state,but ddseq is wrong,discarded.",
                            naddr
                        ));
                        neighbor::event::send(iaddr, naddr, neighbor::event::Event::SeqNumberMismatch).await;
                        return ;
                    }
                    // just receive the dd_packet
                    neighbor::set_ddseqno(iaddr, naddr, packet.dd_sequence_number + 1).await;
                    neighbor::save_last_dd(iaddr, naddr, packet.clone()).await;
                }
                neighbor::status::Status::Loading => {
                    crate::util::error(&format!(
                        "Received DD packet from neighbor {} in Loading state",
                        naddr
                    ));
                }
                neighbor::status::Status::Full => {
                    crate::util::log(&format!(
                        "Received DD packet from neighbor {} in Full state",
                        naddr
                    ));
                    let mut status = neighbor::get_status(iaddr, naddr).await;
                    if packet.is_flag_i_set() {
                        status = neighbor::status::Status::Init;
                    }
                    if packet.is_flag_m_set() {
                        status = neighbor::status::Status::TwoWay;
                    }
                    if packet.is_flag_ms_set() {
                        status = neighbor::status::Status::ExStart;
                    }
                    neighbor::set_status(iaddr, naddr, status).await;
                }

                _ => {}
            }
        }
    }
}
