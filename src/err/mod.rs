use std::future::Future;

pub struct OspfError {
    pub message: String,
}

impl OspfError {
    pub fn new(message: String) -> Self {
        OspfError { message }
    }
}

impl std::fmt::Debug for OspfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::fmt::Display for OspfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for OspfError {
    fn description(&self) -> &str {
        &self.message
    }
}

unsafe impl Send for OspfError {}
unsafe impl Sync for OspfError {}
