// src/state.rs

/// Represents the state of a Styx connection.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ConnectionState {
    /// The connection is closed.
    Closed,
    /// The server is waiting for a connection.
    Listen,
    /// The client has sent a SYN packet and is waiting for a SYN-ACK.
    SynSent,
    /// The server has received a SYN and sent a SYN-ACK, waiting for the final ACK.
    SynReceived,
    /// The handshake is complete and the connection is ready for data transfer.
    Established,
}
