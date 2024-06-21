#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterfaceStatus {
    Down,
    Loopback,
    Waiting,
    PointToPoint,
    DRother,
    Backup,
    Question,
    DR,
}

impl InterfaceStatus {
    pub fn change_to(&mut self, event: super::event::InterfaceEvent) {
        match event {
            super::event::InterfaceEvent::LoopInd => {
                // here may be wrong
                if self != &InterfaceStatus::Down {
                    crate::error("LoopInd event received on non-down interface.");
                    return;
                }
                *self = InterfaceStatus::Loopback;
            }
            super::event::InterfaceEvent::UnloopInd => {
                if self != &InterfaceStatus::Loopback {
                    crate::error("UnloopInd event received on non-loopback interface.");
                    return;
                }
                *self = InterfaceStatus::Down;
            }
            super::event::InterfaceEvent::WaitTimer | super::event::InterfaceEvent::BackupSeen => {
                if self != &InterfaceStatus::Waiting {
                    crate::error("BackupSeen event received on non-waiting interface.");
                    return;
                }
                *self = InterfaceStatus::Question;
            }
            super::event::InterfaceEvent::NeighborChange(status) => {
                if self != &InterfaceStatus::Question {
                    crate::error("NeighborChange event received on non-question interface.");
                    return;
                }
                match status {
                    InterfaceStatus::DR => {
                        *self = InterfaceStatus::DRother;
                    }
                    InterfaceStatus::DRother => {
                        *self = InterfaceStatus::DR;
                    }
                    InterfaceStatus::Backup => {
                        *self = InterfaceStatus::DRother;
                    }
                    _ => {
                        crate::error("NeighborChange event received with invalid status.");
                    }
                }
            }
            super::event::InterfaceEvent::InterfaceUp => {
                if self != &InterfaceStatus::Down {
                    crate::error("InterfaceUp event received on non-down interface.");
                    return;
                }
                *self = InterfaceStatus::Waiting;
            }
            super::event::InterfaceEvent::InterfaceDown => {
                //not implemented fully
                *self = InterfaceStatus::Down;
            }
        }
    }
}
