use std::fmt;

#[derive(Debug)]
pub enum GovernanceError {
    NoAgents,
    InvalidConfig(String),
}

impl fmt::Display for GovernanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GovernanceError::NoAgents => write!(f, "no agents to allocate budget for"),
            GovernanceError::InvalidConfig(msg) => write!(f, "invalid config: {}", msg),
        }
    }
}

impl std::error::Error for GovernanceError {}
