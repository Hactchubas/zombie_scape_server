use serde::{Deserialize, Serialize};
use zombie_scape::{GameConfig, GameStatus};

/// Client → Server messages
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    CreateSession { config: GameConfig },
    StepSimulation { session_id: String, steps: u32 },
    GetState { session_id: String },
    CloseSession { session_id: String },
}

/// Server → Client messages
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    SessionCreated {
        session_id: String,
        initial_state: GameStateSnapshot,
        maze_grid: Vec<Vec<String>>,  // Send full maze grid only once
    },
    StateUpdate {
        session_id: String,
        state: GameStateSnapshot,
    },
    Error {
        message: String,
        code: String,
    },
}

/// Serializable game state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateSnapshot {
    pub step: u64,
    pub status: GameStatus,
    pub fugitive: AgentSnapshot,
    pub zombies: Vec<AgentSnapshot>,
    pub maze_info: MazeInfo,
}

/// Serializable agent snapshot with type discrimination
///
/// Uses a tagged enum to differentiate between fugitive and zombie agents,
/// allowing zombie-specific visualization data (vision cones, paths, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "agent_type", rename_all = "snake_case")]
pub enum AgentSnapshot {
    /// Fugitive agent snapshot
    Fugitive {
        position: [f32; 2],
        velocity: [f32; 2],
        /// Optional: Current path being followed (A* waypoints)
        #[serde(skip_serializing_if = "Option::is_none")]
        current_path: Option<Vec<[f32; 2]>>,
        vision_angle: f32,
        vision_range: f32,
    },
    /// Zombie agent snapshot with debug visualization data
    Zombie {
        position: [f32; 2],
        velocity: [f32; 2],
        /// Current FSM state ("wander" or "pursuit")
        state: String,
        /// Vision range in world units
        vision_range: f32,
        /// Vision cone half-angle in radians
        vision_angle: f32,
        /// Last known position of fugitive (for visualization)
        #[serde(skip_serializing_if = "Option::is_none")]
        last_seen_position: Option<[f32; 2]>,
        /// Current Dijkstra path being followed
        #[serde(skip_serializing_if = "Option::is_none")]
        current_path: Option<Vec<[f32; 2]>>,
    },
}

/// Maze information for clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MazeInfo {
    pub width: usize,
    pub height: usize,
    pub cell_size: f32,
    pub start_position: [f32; 2],
    pub exit_position: [f32; 2],
}
