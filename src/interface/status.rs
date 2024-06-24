use std::fmt::Debug;


#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Status {
    Down,
    Loopback,
    Waiting,
    PointToPoint,
    DRother,
    Backup,
    Question,
    DR,
}

impl Debug for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Down => write!(f, "Down"),
            Status::Loopback => write!(f, "Loopback"),
            Status::Waiting => write!(f, "Waiting"),
            Status::PointToPoint => write!(f, "PointToPoint"),
            Status::DRother => write!(f, "DRother"),
            Status::Backup => write!(f, "Backup"),
            Status::Question => write!(f, "Question"),
            Status::DR => write!(f, "DR"),
        }
    }
}

/// the status machine for the interface
pub async fn status_changed(
    interface_name: String,
    event: super::event::Event,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match event {
        super::event::Event::InterfaceUp => {
            let interface_name_map = super::INTERFACES_BY_NAME.read().await;
            if let Some(interface) = interface_name_map.get(&interface_name.clone()) {
                let mut interface = interface.write().await;
                let network_type = interface.network_type;
                let router_priority = interface.router_priority;
                let it_ipv4_addr = interface.ip;
                if let super::status::Status::Down = interface.status {
                    tokio::spawn(crate::interface::handle::init_when_interface_up(
                        it_ipv4_addr.clone(),
                        interface_name.clone(),
                        network_type,
                        router_priority,
                    ));
                    match interface.network_type {
                        super::NetworkType::Broadcast | super::NetworkType::NBMA => {
                            if interface.router_priority == 0 {
                                interface.status = super::status::Status::DRother;
                            } else {
                                interface.status = super::status::Status::Waiting;
                            }
                        }
                        super::NetworkType::PointToMultipoint
                        | super::NetworkType::PointToPoint
                        | super::NetworkType::VirtualLink => {
                            interface.status = super::status::Status::PointToPoint;
                        }
                    }
                    crate::util::debug(&format!(
                        "Interface {} status turned {:#?}",
                        interface_name, interface.status
                    ));
                } else {
                    crate::util::error(&format!(
                        "Interface {}'status is not down ,can not turn up.",
                        interface_name
                    ));
                    return Err(Box::new(crate::err::OspfError::new(
                        "Interface status is not down,can not turn up.".to_string(),
                    )));
                }
            } else {
                crate::util::error(&format!("Interface {} not found", interface_name));
                return Err(Box::new(crate::err::OspfError::new(
                    "Interface not found".to_string(),
                )));
            }
        }
        super::event::Event::InterfaceDown => {}
        super::event::Event::LoopInd => {}
        super::event::Event::UnloopInd => {
            let interface_name_map = super::INTERFACES_BY_NAME.read().await;
            if let Some(interface) = interface_name_map.get(&interface_name).cloned() {
                let mut interface = interface.write().await;
                if let super::status::Status::Loopback = interface.status {
                    interface.status = super::status::Status::Down;
                    crate::util::debug(&format!(
                        "Interface {} status turned {:#?}",
                        interface_name, interface.status
                    ));
                } else {
                    crate::util::error(&format!(
                        "Interface {}'status is not loopback ,can not turn unloop.",
                        interface_name
                    ));
                    return Err(Box::new(crate::err::OspfError::new(
                        "Interface status is not loopback,can not turn unloop.".to_string(),
                    )));
                }
            } else {
                crate::util::error(&format!("Interface {} not found", interface_name));
                return Err(Box::new(crate::err::OspfError::new(
                    "Interface not found".to_string(),
                )));
            }
        }
        super::event::Event::WaitTimer => {}
        super::event::Event::NeighborChange => {}
        super::event::Event::BackupSeen => {}
        _ => {
            crate::util::error("Invalid super::event type,ignored.");
            return Err(Box::new(crate::err::OspfError::new(
                "Invalid super::event type,ignored.".to_string(),
            )));
        }
    }
    Ok(())
}
