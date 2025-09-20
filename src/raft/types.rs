use std::any::Any;
use std::fmt;

#[derive(PartialEq, Debug)]
pub enum CMState {
    Follower,
    Candidate,
    Leader,
    Dead,
}

impl fmt::Display for CMState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CMState::Follower => write!(f, "Follower"),
            CMState::Candidate => write!(f, "Candidate"),
            CMState::Leader => write!(f, "Leader"),
            CMState::Dead => write!(f, "Dead"),
        }
    }
}

pub struct CommitEntry {
    // pub command: Box<dyn Any>,
    pub index: i64,
    pub term: i64,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    // pub command: Box<dyn Any>,
    pub term: i64,
}