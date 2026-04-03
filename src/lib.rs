//! pinchtask library root.
//!
//! This crate provides an MCP (Model Context Protocol) task management server,
//! inspired by mcp-shrimp-task-manager.

pub mod cli;
pub mod core;
pub mod models;

pub mod server;
pub mod store;
pub mod tools;
