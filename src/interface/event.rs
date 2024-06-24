use std::fmt::Debug;
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Event {
    InterfaceUp,
    WaitTimer,
    BackupSeen,
    NeighborChange,
    LoopInd,
    UnloopInd,
    InterfaceDown,
}

unsafe impl Send for Event {}

impl Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::InterfaceUp => write!(f, "InterfaceUp"),
            Event::WaitTimer => write!(f, "WaitTimer"),
            Event::BackupSeen => write!(f, "BackupSeen"),
            Event::NeighborChange => write!(f, "NeighborChange"),
            Event::LoopInd => write!(f, "LoopInd"),
            Event::UnloopInd => write!(f, "UnloopInd"),
            Event::InterfaceDown => write!(f, "InterfaceDown"),
        }
    }
}
