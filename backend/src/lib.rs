//! # Rune — Rust Autonomous Agent Framework
//!
//! Rune is a production-grade autonomous AI agent framework. The library
//! crate exposes every subsystem (config, LLM router, agent loop, tools,
//! memory, REST API, Telegram bot) as public modules so that they can be
//! unit-tested in isolation and embedded inside the `rune` binary defined
//! in `src/main.rs`.

#![warn(missing_docs)]

pub mod agent;
pub mod config;
pub mod core;
pub mod error;
pub mod interfaces;
pub mod state;
pub mod tools;
pub mod utils;
