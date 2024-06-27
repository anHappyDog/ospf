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
        if dd_packet.interface_mtu > crate::IPV4_PACKET_MTU as u16 {
            crate::util::error("interface mtu is too large in the received dd packet.");
            return;
        }
        let g_neighbors = neighbor::NEIGHBORS.read().await;
        let neighbors = g_neighbors.get(&int_ipv4_addr).unwrap().read().await;
        let neighbor = neighbors.get(&packet_source_addr).unwrap().read().await;
        let mut status = neighbor.state.clone();
        let master = neighbor.master;
        let former_dd_seq = neighbor.dd_seq;
        drop(neighbor);
        drop(neighbors);
        drop(g_neighbors);
        loop {
            match status {
                neighbor::status::Status::Full => {
                    crate::util::debug("received dd packet from full state neighbor.");
                    let g_neighbors = neighbor::NEIGHBORS.read().await;
                    let neighbors = g_neighbors.get(&int_ipv4_addr).unwrap().read().await;
                    let neighbor = neighbors.get(&packet_source_addr).unwrap().write().await;
                    match neighbor.last_dd.clone() {
                        Some(d) => {
                            if d.dd_sequence_number == dd_packet.dd_sequence_number
                                && d.options == dd_packet.options
                            {
                                if !master {
                                    crate::util::debug("received the same dd packet,ignored.");
                                    return;
                                } else {
                                    // interface is master
                                }
                            }
                        }
                        None => {
                            neighbor::event::send(
                                packet_source_addr,
                                neighbor::event::Event::SeqNumberMismatch,
                            )
                            .await;
                            crate::util::error("received dd packet from full state neighbor, but the last dd packet is none.");
                        }
                    }
                    return;
                }
                neighbor::status::Status::Loading => {
                    crate::util::debug("received dd packet from loading state neighbor.");
                    let g_neighbors = neighbor::NEIGHBORS.read().await;
                    let neighbors = g_neighbors.get(&int_ipv4_addr).unwrap().read().await;
                    let neighbor = neighbors.get(&packet_source_addr).unwrap().write().await;
                    match neighbor.last_dd.clone() {
                        Some(d) => {
                            if d.dd_sequence_number == dd_packet.dd_sequence_number
                                && d.options == dd_packet.options
                            {
                                if !master {
                                    crate::util::debug("received the same dd packet,ignored.");
                                    return;
                                } else {
                                    // interface is master
                                }
                            }
                        }
                        None => {
                            neighbor::event::send(
                                packet_source_addr,
                                neighbor::event::Event::SeqNumberMismatch,
                            )
                            .await;
                            crate::util::error("received dd packet from loading state neighbor, but the last dd packet is none.");
                        }
                    }
                    return;
                }
                neighbor::status::Status::Down => {
                    // done
                    crate::util::debug("received dd packet from down state neighbor,rejected.");
                    return;
                }
                neighbor::status::Status::Init => {
                    crate::util::debug("received dd packet from init state neighbor.");
                    neighbor::event::send(
                        packet_source_addr,
                        neighbor::event::Event::TwoWayReceived,
                    )
                    .await;
                    return;
                }
                neighbor::status::Status::TwoWay => {
                    crate::util::debug("received dd packet from two way state neighbor,ignored.");
                    let g_neighbors = neighbor::NEIGHBORS.read().await;
                    let neighbors = g_neighbors.get(&int_ipv4_addr).unwrap().read().await;
                    let neighbor = neighbors.get(&packet_source_addr).unwrap().read().await;
                    status = neighbor.state.clone();
                    drop(neighbor);
                    drop(neighbors);
                    drop(g_neighbors);
                    if status == neighbor::status::Status::ExStart {
                        continue;
                    }
                    return;
                }
                neighbor::status::Status::ExStart => {
                    crate::util::debug("received dd packet from ex start state neighbor.");
                    if dd_packet.lsa_headers.is_empty()
                        && dd_packet.is_flag_i_set()
                        && dd_packet.is_flag_m_set()
                        && dd_packet.is_flag_ms_set()
                        && dd_packet.header.router_id > crate::ROUTER_ID.clone()
                    {
                        let g_neighbors = neighbor::NEIGHBORS.read().await;
                        let neighbors = g_neighbors.get(&int_ipv4_addr).unwrap().write().await;
                        let mut neighbor =
                            neighbors.get(&packet_source_addr).unwrap().write().await;
                        neighbor.master = true;
                        neighbor.dd_seq = dd_packet.dd_sequence_number;
                        drop(neighbor);
                        drop(neighbors);
                        drop(g_neighbors);
                        neighbor::event::send(
                            packet_source_addr,
                            neighbor::event::Event::NegotiationDone,
                        )
                        .await;
                    }
                    if !dd_packet.is_flag_i_set()
                        && !dd_packet.is_flag_ms_set()
                        && dd_packet.dd_sequence_number == former_dd_seq
                    {
                        let g_neighbors = neighbor::NEIGHBORS.read().await;
                        let neighbors = g_neighbors.get(&int_ipv4_addr).unwrap().write().await;
                        let mut neighbor =
                            neighbors.get(&packet_source_addr).unwrap().write().await;
                        neighbor.master = false;
                        drop(neighbor);
                        drop(neighbors);
                        drop(g_neighbors);
                        neighbor::event::send(
                            packet_source_addr,
                            neighbor::event::Event::NegotiationDone,
                        )
                        .await;
                    }
                    return;
                }
                neighbor::status::Status::Exchange => {
                    crate::util::debug("received dd packet from exchange state neighbor.");
                    let g_neighbors = neighbor::NEIGHBORS.read().await;
                    let neighbors = g_neighbors.get(&int_ipv4_addr).unwrap().read().await;
                    let mut neighbor = neighbors.get(&packet_source_addr).unwrap().write().await;
                    let former_option = if let Some(p) = &neighbor.last_dd {
                        if dd_packet.dd_sequence_number == p.dd_sequence_number && master {
                            crate::util::debug("received the same dd packet,ignored.");
                            return;
                        }
                        p.options
                    } else {
                        dd_packet.options
                    };
                    neighbor.last_dd = Some(dd_packet.clone());
                    drop(neighbor);
                    drop(neighbors);
                    drop(g_neighbors);
                    if dd_packet.is_flag_ms_set() != master {
                        neighbor::event::send(
                            packet_source_addr,
                            neighbor::event::Event::SeqNumberMismatch,
                        )
                        .await;

                        return;
                    }
                    if dd_packet.is_flag_i_set() {
                        neighbor::event::send(
                            packet_source_addr,
                            neighbor::event::Event::SeqNumberMismatch,
                        )
                        .await;

                        return;
                    }
                    if dd_packet.options != former_option {
                        neighbor::event::send(
                            packet_source_addr,
                            neighbor::event::Event::SeqNumberMismatch,
                        )
                        .await;

                        return;
                    }
                    if master && dd_packet.dd_sequence_number != former_dd_seq {
                        neighbor::event::send(
                            packet_source_addr,
                            neighbor::event::Event::SeqNumberMismatch,
                        )
                        .await;

                        return;
                    } else if !master && dd_packet.dd_sequence_number == former_dd_seq + 1 {
                        neighbor::event::send(
                            packet_source_addr,
                            neighbor::event::Event::SeqNumberMismatch,
                        )
                        .await;

                        return;
                    } else {
                        //accept the received dd packet
                    }
                    return;
                }
                neighbor::status::Status::Attempt => {
                    //done
                    crate::util::debug("received dd packet from attempt state neighbor,rejected.");
                    return;
                }
                _ => {}
            }
        }
    }
}
