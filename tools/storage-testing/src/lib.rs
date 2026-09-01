//! Disposable-fixture test harness.
//!
//! This crate is deliberately separate from the desktop application.  It owns
//! the constrained loop-fixture lifecycle used by the integration lab, while
//! product actions remain typed UDisks operations.

pub mod artifacts;
pub mod cmd;
pub mod errors;
pub mod harness;
pub mod lab;
pub mod ledger;
pub mod runtime;
pub mod spec;
