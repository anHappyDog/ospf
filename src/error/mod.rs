pub struct RouterNotSetError;

impl std::error::Error for RouterNotSetError {}

impl std::fmt::Display for RouterNotSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Router not set")
    }
}

impl std::fmt::Debug for RouterNotSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Router not set")
    }
}
