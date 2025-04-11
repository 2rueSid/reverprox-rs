// Message is a data that is transferred between client and server.
// In this system message is represented as Bytes.
//
// First message comes to the server from the client; it's a message that initiates a connection.
// It includes:
// - Message Start Bit (magic)
// - Protocol Version
// - Message Type (Initial)
// - Connection ID
// - Message ID
// - Connection details:
//   - Client IP and Port
//   - Target host and port the client is proxying to
// - Payload length and data
//
// After the connection is initialized, all messages from that client will include:
// - Message Start Bit (magic)
// - Protocol Version
// - Message Type (Data, Close, Ping, etc.)
// - Connection ID
// - Message ID
// - Payload length
// - Payload
//
// Since the conenction is always bidirectional, server will have the same structure of the
// message.
//
// It's important to note that message format is constant in structure and length of the payload is
// defined explicitly via `length` field.
//
// If the message payload exceeds the predefined constant `CHUNK_SIZE`, it will be split into chunks.
// A message is considered to be processed **only when the full payload has been received**, as
// defined by the `length`.
//
// This protocol works both ways — from client to server and from server to client.
use std::{
    io::{self, ErrorKind},
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use bytes::Bytes;
use uuid::Uuid;

#[path = "utils.rs"]
pub mod msg_utils;

/// The maximum size of a single chunk of data in bytes.
pub const CHUNK_SIZE: usize = 512;

/// Message Start Bit used to identify the beginning of a message frame.
pub const MAGIC_BYTE: u8 = 0xAA;

/// Lenght of the fields magic-lenght
pub const HEADER_LENGTH: usize = 39;

/// Represents the type of the message transferred between server and client.
/// It is used to determine how to decode the payload and how to route the logic.
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum MessageType {
    /// Used when establishing a new connection.
    Initial = 0x1,

    /// Used to transfer proxy data.
    Data = 0x2,

    /// Used to signal that a connection should be closed.
    Close = 0x3,

    /// Used to check if the connection is alive.
    Ping = 0x4,
}

/// Represents the version of the QUIC protocol used in the system.
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ProtocolVersion {
    /// Current and only supported version.
    V1 = 0x1,
}

/// The core protocol unit that is transmitted through the QUIC stream.
/// It contains all metadata and the payload needed to process a client-server exchange.
#[derive(Debug, Clone)]
pub struct Message {
    /// Message Start Bit; fixed length = 1 byte; used to identify the beginning of a message.
    pub magic: u8,

    /// Protocol Version; fixed length = 1 byte; current version defined in [`ProtocolVersion`].
    pub version: ProtocolVersion,

    /// Type of the message; defines how the payload should be interpreted. len = 1 byte
    pub message_type: MessageType,

    /// ID that identifies the connection; fixed length = 16 bytes - UUIDv4; same for all messages on a virtual tunnel.
    pub connection_id: Uuid,

    /// ID that identifies the message; fixed length = 16 bytes, UUIDv4;
    pub message_id: Uuid,

    /// Payload length in bytes; fixed length = 4 bytes; used to determine how many bytes to read after header.
    pub length: u32,

    /// Actual Payload; variable length = N; interpretation depends on `message_type`.
    pub payload: Bytes,
}

impl Message {
    pub fn new(msg_type: MessageType, connection_id: Uuid, payload: Bytes) -> Message {
        Message {
            magic: MAGIC_BYTE,
            version: ProtocolVersion::V1,
            message_type: msg_type,
            connection_id,
            message_id: msg_utils::generate_uuid(),
            length: payload.len() as u32,
            payload,
        }
    }

    pub fn encode(&self) -> Bytes {
        let mut buffer = Vec::with_capacity(HEADER_LENGTH + self.payload.len());

        buffer.push(self.magic);
        buffer.push(self.version as u8);
        buffer.push(self.message_type as u8);
        buffer.extend_from_slice(self.connection_id.as_bytes());
        buffer.extend_from_slice(self.message_id.as_bytes());
        buffer.extend_from_slice(&self.length.to_be_bytes());
        buffer.extend_from_slice(&self.payload);

        Bytes::from(buffer)
    }

    pub fn decode(msg: &Bytes) -> io::Result<Message> {
        if msg.len() < HEADER_LENGTH {
            return Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                "Headers are incomplete",
            ));
        }

        let magic = msg[0];
        let version = match msg[1] {
            0x1 => ProtocolVersion::V1,
            _ => {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "Unknown message type",
                ));
            }
        };
        let message_type = match msg[2] {
            0x1 => MessageType::Initial,
            0x2 => MessageType::Data,
            0x3 => MessageType::Close,
            0x4 => MessageType::Ping,
            _ => {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "Unknown message type",
                ));
            }
        };
        let connection_id = match Uuid::from_slice(&msg[3..19]) {
            Ok(uuid) => uuid,
            Err(err) => {
                return Err(io::Error::new(ErrorKind::InvalidData, err));
            }
        };

        let message_id = match Uuid::from_slice(&msg[19..35]) {
            Ok(uuid) => uuid,
            Err(err) => {
                return Err(io::Error::new(ErrorKind::InvalidData, err));
            }
        };
        let length = u32::from_be_bytes(msg[35..39].try_into().unwrap());

        if msg.len() < HEADER_LENGTH + length as usize {
            return Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                "Payload incomplete",
            ));
        }

        let payload = Bytes::from(msg[HEADER_LENGTH..HEADER_LENGTH + length as usize].to_vec());

        Ok(Message {
            magic,
            version,
            message_type,
            message_id,
            connection_id,
            length,
            payload,
        })
    }
}

/// A payload structure that appears only in the [`MessageType::Initial`] message.
/// It contains metadata required to associate a client with a target server to proxy.
#[derive(Debug, Clone, Copy)]
pub struct InitializationMessage {
    /// IPv4 address of the client
    pub client_ip: Ipv4Addr,

    /// Port on which the client runs the QUIC connection
    pub client_port: u16,

    /// Local port on the client machine the server will proxy data to
    pub proxy_port: u16,

    /// Local host on the client machine the server will proxy data to
    pub proxy_host: Ipv4Addr,
}

impl InitializationMessage {
    pub fn new(addr: SocketAddr, proxy_addr: SocketAddr) -> io::Result<InitializationMessage> {
        if !addr.is_ipv4() || !proxy_addr.is_ipv4() {
            return Err(io::Error::new(
                ErrorKind::Unsupported,
                "IPv6 is not supported",
            ));
        }

        let ipv4 = match addr.ip() {
            IpAddr::V4(ipv4) => ipv4,
            _ => unreachable!(),
        };

        let proxy_ipv4 = match proxy_addr.ip() {
            IpAddr::V4(ipv4) => ipv4,
            _ => unreachable!(),
        };

        Ok(InitializationMessage {
            client_ip: ipv4,
            client_port: addr.port(),
            proxy_port: proxy_addr.port(),
            proxy_host: proxy_ipv4,
        })
    }

    pub fn encode(&self) -> Bytes {
        let mut buffer = Vec::with_capacity(12);

        buffer.extend_from_slice(&self.client_port.to_be_bytes());
        buffer.extend_from_slice(&self.proxy_port.to_be_bytes());
        buffer.extend_from_slice(&self.client_ip.octets());
        buffer.extend_from_slice(&self.proxy_host.octets());

        Bytes::from(buffer)
    }

    pub fn decode(msg: &Bytes) -> io::Result<InitializationMessage> {
        if msg.len() < 12 {
            return Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                "Initial message is incorrect",
            ));
        }

        let client_port = u16::from_be_bytes(msg[0..2].try_into().unwrap());
        let proxy_port = u16::from_be_bytes(msg[2..4].try_into().unwrap());

        let client_ip = Ipv4Addr::from_bits(u32::from_be_bytes(msg[4..8].try_into().unwrap()));
        let proxy_host = Ipv4Addr::from_bits(u32::from_be_bytes(msg[8..12].try_into().unwrap()));

        Ok(InitializationMessage {
            client_ip,
            client_port,
            proxy_port,
            proxy_host,
        })
    }
}
