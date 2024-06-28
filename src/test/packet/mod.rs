use std::net;

use pnet::{packet::{ip::{IpNextHeaderProtocol, IpNextHeaderProtocols}, Packet}, transport::{self, TransportChannelType}};
use socket2::{Domain, Protocol, Socket, Type};
use tokio::{runtime::Runtime, task::block_in_place};

use crate::{packet::ospf_packet_checksum, OPTION_E};



