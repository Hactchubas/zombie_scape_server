//! WebSocket protocol definitions for the zombie escape game server
//!
//! This module defines the JSON message format for client-server communication.

pub mod messages;

pub use messages::{
    ClientMessage, ServerMessage, GameStateSnapshot, AgentSnapshot, MazeInfo,
};
