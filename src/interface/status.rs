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



