// src/lib.rs

/// This file makes the 'packet' module available as a library.
/// Binaries like 'client' and 'server' can then use it.
pub mod packet;
pub mod state;
pub mod styx_socket;
