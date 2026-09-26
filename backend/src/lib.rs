//! Customer-support ticket system backend, laid out as Ports & Adapters.
//!
//! Dependencies point inward only:
//!
//! ```text
//! adapters::inbound (HTTP) ──▶ application::ports::inbound  (use-case traits)
//!                                     ▲ implemented by
//!                              application::services ──▶ domain
//!                                     │ calls
//!                              application::ports::outbound (repo/probe traits)
//!                                     ▲ implemented by
//!                              adapters::outbound (Postgres)
//! ```
//!
//! `domain` and `application` must never import axum, sqlx or other I/O crates.
//! `wiring.rs` is the composition root that wires adapters into services.

pub mod adapters;
pub mod application;
pub mod config;
pub mod domain;
pub mod wiring;
