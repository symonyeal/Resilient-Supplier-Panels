//! Certified resilient qualification coverage on the USDA Qualified Bidders List.
//!
//! The core (bits … quotient) computes on index sets and imports no I/O; the shell
//! (data, documents, bench, report, site, the CLI) reads inputs and writes outputs.

pub mod audit;
pub mod bench;
pub mod bits;
pub mod catalog;
pub mod certify;
pub mod change;
pub mod contracts;
pub mod data;
pub mod entities;
pub mod gaps;
pub mod instance;
pub mod json;
pub mod mmcs;
pub mod portfolio;
pub mod pyrand;
pub mod qbl;
pub mod quotient;
pub mod report;
pub mod run;
pub mod solve;
pub mod thesis;
pub mod verdicts;
pub mod xlsx;

/// The thesis revision whose algorithms `thesis` ports (Hypergraph-Dualization).
pub const THESIS_REV: &str = "1bb9de48ab91629a7ae9d97488c2181b4098806d";
