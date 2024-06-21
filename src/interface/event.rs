pub enum InterfaceEvent {
    InterfaceUp,
    WaitTimer,
    BackupSeen,
    NeighborChange(super::status::InterfaceStatus),
    LoopInd,
    UnloopInd,
    InterfaceDown,
}

impl InterfaceEvent {}
