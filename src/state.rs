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
    /// The active closer has sent a FIN and is waiting for an ACK.
    FinWait1,
    /// The active closer has received an ACK for its FIN and is waiting for the peer's FIN.
    FinWait2,
    /// The passive closer has received a FIN and will send its own FIN after the application closes.
    CloseWait,
    /// The passive closer has sent its FIN and is waiting for the final ACK.
    LastAck,
    /// The active closer waits for a short period to ensure the final ACK was received.
    TimeWait,
}
