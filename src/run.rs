//! Limits of one enumeration run: a wall-clock deadline, checked cooperatively by the routes.

use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct Clock {
    pub t0: Instant,
    pub deadline: Option<Instant>,
}

impl Clock {
    pub fn unlimited() -> Self {
        Clock { t0: Instant::now(), deadline: None }
    }

    pub fn limit(seconds: f64) -> Self {
        let t0 = Instant::now();
        Clock { t0, deadline: Some(t0 + Duration::from_secs_f64(seconds)) }
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
}

impl Stop {
    pub fn name(self) -> &'static str {
        match self {
            Stop::Timeout => "timeout",
            Stop::Cap => "cap",
            Stop::Memory => "memory",
        }
    }
}
