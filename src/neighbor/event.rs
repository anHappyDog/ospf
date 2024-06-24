use std::fmt::Debug;

pub enum Event {
    HelloReceived,
    Start,
    TwoWayReceived,
    NegotiationDone,
    ExchangeDone,
    BadLSReq,
    LoadingDone,
    AdjOk,
    SeqNumberMismatch,
    OneWayReceived,
    KillNbr,
    InactivityTimer,
    LLDown,
}

impl Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::HelloReceived => write!(f, "HelloReceived"),
            Event::Start => write!(f, "Start"),
            Event::TwoWayReceived => write!(f, "TwoWayReceived"),
            Event::NegotiationDone => write!(f, "NegotiationDone"),
            Event::ExchangeDone => write!(f, "ExchangeDone"),
            Event::BadLSReq => write!(f, "BadLSReq"),
            Event::LoadingDone => write!(f, "LoadingDone"),
            Event::AdjOk => write!(f, "AdjOk"),
            Event::SeqNumberMismatch => write!(f, "SeqNumberMismatch"),
            Event::OneWayReceived => write!(f, "OneWayReceived"),
            Event::KillNbr => write!(f, "KillNbr"),
            Event::InactivityTimer => write!(f, "InactivityTimer"),
            Event::LLDown => write!(f, "LLDown"),
        }
    }
}
