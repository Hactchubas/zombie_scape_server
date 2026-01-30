mod protocol;

use futures_util::{SinkExt, StreamExt};
use protocol::{AgentSnapshot, ClientMessage, GameStateSnapshot, MazeInfo, ServerMessage};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;
use zombie_scape::{FugitiveSnapshot, GameConfig, GameState, ZombieSnapshot};

/// Game session wrapper
struct GameSession {
    id: String,
    state: GameState,
}

impl GameSession {
    fn new(config: GameConfig) -> Self {
        let id = Uuid::new_v4().to_string();
        let state = GameState::new(config);

        GameSession { id, state }
    }

    fn step(&mut self, steps: u32) {
        const DT: f32 = 0.016; // ~60 FPS timestep
        for _ in 0..steps {
            self.state.step(DT);
        }
    }

    fn get_snapshot(&self) -> GameStateSnapshot {
        // Convert fugitive to AgentSnapshot::Fugitive
        let fugitive_snapshot =
            FugitiveSnapshot::from_agent(&self.state.fugitive, &self.state.graph);
        let fugitive = AgentSnapshot::Fugitive {
            position: fugitive_snapshot.position,
            velocity: fugitive_snapshot.velocity,
            current_path: fugitive_snapshot.current_path, // TODO: Add fugitive path if needed for visualization
            vision_range: fugitive_snapshot.vision_range,
            vision_angle: fugitive_snapshot.vision_angle,
        };

        // Convert zombies to AgentSnapshot::Zombie with debug data
        let zombie_snapshots: Vec<AgentSnapshot> = self
            .state
            .zombies
            .iter()
            .map(|z| {
                let zs = ZombieSnapshot::from_agent(z, &self.state.graph);
                AgentSnapshot::Zombie {
                    position: zs.position,
                    velocity: zs.velocity,
                    state: zs.state,
                    vision_range: zs.vision_range,
                    vision_angle: zs.vision_angle,
                    last_seen_position: zs.last_seen_position,
                    current_path: zs.current_path,
                }
            })
            .collect();

        let start_pos = self.state.start_position();
        let exit_pos = self.state.exit_position();

        GameStateSnapshot {
            step: self.state.current_step,
            status: self.state.status,
            fugitive,
            zombies: zombie_snapshots,
            maze_info: MazeInfo {
                width: self.state.config.maze_width,
                height: self.state.config.maze_height,
                cell_size: self.state.config.cell_size,
                start_position: [start_pos.x(), start_pos.y()],
                exit_position: [exit_pos.x(), exit_pos.y()],
            },
        }
    }
}

/// Session registry (for Milestone 3, currently single session)
type SessionRegistry = Arc<Mutex<HashMap<String, Arc<Mutex<GameSession>>>>>;

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");

    println!("🎮 Zombie Escape Server listening on {}", addr);
    println!("📝 Milestone 1: Single fugitive navigation");
    println!();

    let sessions: SessionRegistry = Arc::new(Mutex::new(HashMap::new()));

    while let Ok((stream, peer)) = listener.accept().await {
        println!("🔌 New connection from {}", peer);
        let sessions = Arc::clone(&sessions);
        tokio::spawn(handle_connection(stream, sessions, peer.to_string()));
    }
}

async fn handle_connection(stream: TcpStream, sessions: SessionRegistry, peer: String) {
    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("❌ WebSocket handshake error with {}: {}", peer, e);
            return;
        }
    };

    println!("✅ WebSocket connection established with {}", peer);

    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                println!("📨 Received from {}: {}", peer, text);

                // Parse client message
                let response = match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => handle_client_message(client_msg, &sessions).await,
                    Err(e) => ServerMessage::Error {
                        message: format!("Invalid JSON: {}", e),
                        code: "parse_error".to_string(),
                    },
                };

                // Send response
                let response_json = serde_json::to_string(&response).unwrap();
                println!("📤 Sending to {}: {}", peer, response_json);

                if let Err(e) = write.send(Message::Text(response_json.into())).await {
                    eprintln!("❌ Failed to send message to {}: {}", peer, e);
                    break;
                }
            }
            Ok(Message::Close(_)) => {
                println!("👋 Client {} disconnected", peer);
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("❌ WebSocket error with {}: {}", peer, e);
                break;
            }
        }
    }

    println!("🔌 Connection closed with {}", peer);
}

fn serialize_grid(grid: &zombie_scape::Grid2D) -> Vec<Vec<String>> {
    let height = grid.height();
    let width = grid.width();
    let cell_size = grid.cell_size();
    let mut result = Vec::new();

    for y in 0..height {
        let mut row = Vec::new();
        for x in 0..width {
            // Convert grid coordinates to world coordinates (center of cell)
            let world_x = (x as f32 + 0.5) * cell_size;
            let world_y = (y as f32 + 0.5) * cell_size;
            let pos = zombie_scape::Vector2D::from_coords(world_x, world_y);

            let cell_type = if grid.is_walkable(pos) {
                "walkable"
            } else {
                "wall"
            };
            row.push(cell_type.to_string());
        }
        result.push(row);
    }

    result
}

async fn handle_client_message(msg: ClientMessage, sessions: &SessionRegistry) -> ServerMessage {
    match msg {
        ClientMessage::CreateSession { config } => {
            println!("🎮 Creating new session with config: {:?}", config);

            let session = GameSession::new(config);
            let session_id = session.id.clone();
            let initial_state = session.get_snapshot();

            // Serialize the maze grid
            let maze_grid = serialize_grid(&session.state.grid);

            // Store session
            let session_arc = Arc::new(Mutex::new(session));
            sessions
                .lock()
                .unwrap()
                .insert(session_id.clone(), session_arc);

            println!("✅ Session created: {}", session_id);

            ServerMessage::SessionCreated {
                session_id,
                initial_state,
                maze_grid,
            }
        }

        ClientMessage::StepSimulation { session_id, steps } => {
            println!("▶️  Stepping session {} by {} steps", session_id, steps);

            let sessions = sessions.lock().unwrap();

            match sessions.get(&session_id) {
                Some(session_arc) => {
                    let mut session = session_arc.lock().unwrap();
                    session.step(steps);
                    let state = session.get_snapshot();

                    println!("✅ Step {}: Status = {:?}", state.step, state.status);

                    ServerMessage::StateUpdate { session_id, state }
                }
                None => ServerMessage::Error {
                    message: format!("Session not found: {}", session_id),
                    code: "session_not_found".to_string(),
                },
            }
        }

        ClientMessage::GetState { session_id } => {
            let sessions = sessions.lock().unwrap();

            match sessions.get(&session_id) {
                Some(session_arc) => {
                    let session = session_arc.lock().unwrap();
                    let state = session.get_snapshot();

                    ServerMessage::StateUpdate { session_id, state }
                }
                None => ServerMessage::Error {
                    message: format!("Session not found: {}", session_id),
                    code: "session_not_found".to_string(),
                },
            }
        }

        ClientMessage::CloseSession { session_id } => {
            println!("🗑️  Closing session {}", session_id);

            let mut sessions = sessions.lock().unwrap();
            match sessions.remove(&session_id) {
                Some(_) => {
                    println!("✅ Session {} closed", session_id);
                    ServerMessage::StateUpdate {
                        session_id: session_id.clone(),
                        state: GameStateSnapshot {
                            step: 0,
                            status: zombie_scape::GameStatus::Running,
                            fugitive: AgentSnapshot::Fugitive {
                                position: [0.0, 0.0],
                                velocity: [0.0, 0.0],
                                current_path: None,
                                vision_range: 0.0,
                                vision_angle: 0.0
                            },
                            zombies: vec![],
                            maze_info: MazeInfo {
                                width: 0,
                                height: 0,
                                cell_size: 0.0,
                                start_position: [0.0, 0.0],
                                exit_position: [0.0, 0.0],
                            },
                        },
                    }
                }
                None => ServerMessage::Error {
                    message: format!("Session not found: {}", session_id),
                    code: "session_not_found".to_string(),
                },
            }
        }
    }
}
