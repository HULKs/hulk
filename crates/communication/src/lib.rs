//! Communication server and client
//!
//! The communication crate provides a server and client for reading, subscribing, and writing
//! data from and to a running framework. The server and client are designed to be used together.
//! The server listens for incoming connections and dispatches messages to the appropriate sources
//! and sinks. The client connects to the server and sends requests to read, subscribe, or write
//! data.
//!
//! Both the server and client are build on `tokio` for asynchronous I/O.

pub mod client;
pub mod messages;
mod send_or_log;
pub mod server;
