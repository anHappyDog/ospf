#[derive(Clone, Copy,PartialEq, Eq, PartialOrd, Ord)]
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
