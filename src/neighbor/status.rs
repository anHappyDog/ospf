use std::fmt::Debug;

pub enum Status {
    Down,
    Init,
    TwoWay,
    ExStart,
    Exchange,
    Loading,
    Full,
}

impl Status {}

impl Debug for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Down => write!(f, "Down"),
            Status::Init => write!(f, "Init"),
            Status::TwoWay => write!(f, "TwoWay"),
            Status::ExStart => write!(f, "ExStart"),
            Status::Exchange => write!(f, "Exchange"),
            Status::Loading => write!(f, "Loading"),
            Status::Full => write!(f, "Full"),
        }
    }
}
