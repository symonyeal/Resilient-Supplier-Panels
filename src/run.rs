//! Limits of one enumeration run: a wall-clock deadline, checked cooperatively by the routes.

use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct Clock {
    pub t0: Instant,
    pub deadline: Option<Instant>,
}

impl Clock {
    pub fn unlimited() -> Self {
        Clock {
            t0: Instant::now(),
            deadline: None,
        }
    }

    pub fn limit(seconds: f64) -> Self {
        let t0 = Instant::now();
        Clock {
            t0,
            deadline: Some(t0 + Duration::from_secs_f64(seconds)),
        }
    }

    pub fn expired(&self) -> bool {
        self.deadline.is_some_and(|d| Instant::now() >= d)
    }

    pub fn elapsed(&self) -> f64 {
        self.t0.elapsed().as_secs_f64()
    }
}

/// Why a run ended before exhaustion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    Timeout,
    Cap,
    Memory,
    Overflow,
}

impl Stop {
    pub fn name(self) -> &'static str {
        match self {
            Stop::Timeout => "timeout",
            Stop::Cap => "cap",
            Stop::Memory => "memory",
            Stop::Overflow => "overflow",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Complete,
    Stopped(Stop),
    Error,
}
impl Status {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "complete" => Self::Complete,
            "timeout" => Self::Stopped(Stop::Timeout),
            "cap" => Self::Stopped(Stop::Cap),
            "memory" => Self::Stopped(Stop::Memory),
            "overflow" => Self::Stopped(Stop::Overflow),
            "error" => Self::Error,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Stopped(s) => s.name(),
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Berge,
    Fkb,
    Cuts,
    Mmcs,
    Exhaustive,
}
impl Method {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "berge" => Self::Berge,
            "fkb" => Self::Fkb,
            "cuts" => Self::Cuts,
            "mmcs" => Self::Mmcs,
            "exhaustive" => Self::Exhaustive,
            _ => return None,
        })
    }
}
