#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Down,
    Init,
    TwoWay,
    ExStart,
    Exchange,
    Loading,
    Full,
    Attempt,
}
