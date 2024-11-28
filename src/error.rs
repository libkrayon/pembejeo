#[derive(Debug)]
pub enum Error {
    FailedCreatingPembejeo(std::string::String),
}


impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailedCreatingPembejeo(message) => write!(f, "Failed creating pembejeo: {}", message),
        }
    }
}


