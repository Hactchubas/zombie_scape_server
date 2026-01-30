# Zombie Scape Server

A WebSocket server for running Zombie Scape game sessions. This server uses the [zombie_scape](https://github.com/Hactchubas/zombie_scape) library for all the game logic and AI.

Both projects work together: this server handles client connections and session management, while `zombie_scape` provides the actual simulation. This server depends on `zombie_scape`, not the other way around.

## What it does

- Accepts WebSocket connections on `127.0.0.1:8080`
- Creates and manages game sessions
- Steps the simulation forward on request
- Returns game state snapshots as JSON (positions, velocities, paths, vision data, etc.)

## Running

```bash
cargo run
```

The server starts listening for WebSocket connections.

## Web Client

There's a web-based visualizer in the `client/` folder. Just open `client/game_visualizer.html` in your browser while the server is running.

The visualizer lets you:
- Configure maze size, zombie count, seed, and braid probability
- Watch the simulation in real time
- See vision cones, planned paths, and agent trails
- Step through the simulation manually or let it run automatically

## Protocol

The server accepts JSON messages and responds with JSON. Here are the message types:

### Create a session

```json
{
  "type": "create_session",
  "config": {
    "maze_width": 15,
    "maze_height": 15,
    "zombie_count": 3,
    "cell_size": 40.0,
    "braid_probability": 0.3
  }
}
```

Response includes the session ID, initial game state, and the full maze grid.

### Step the simulation

```json
{
  "type": "step_simulation",
  "session_id": "your-session-id",
  "steps": 1
}
```

Returns the updated game state with positions of all agents, their paths, vision cones, and game status.

### Get current state

```json
{
  "type": "get_state",
  "session_id": "your-session-id"
}
```

### Close a session

```json
{
  "type": "close_session",
  "session_id": "your-session-id"
}
```

## Game State Response

The state update includes:
- `step`: Current simulation step
- `status`: "running", "won", or "captured"
- `fugitive`: Position, velocity, current path, vision range/angle
- `zombies`: Array with each zombie's position, velocity, state (Wander/Pursuit), vision data, and last seen position of the fugitive
- `maze_info`: Dimensions, cell size, start and exit positions

## Dependencies

This server depends on the `zombie_scape` library:

```toml
[dependencies]
zombie_scape = { path = "../zombie_scape" }
```

For standalone use, you can change this to use the git repository:

```toml
[dependencies]
zombie_scape = { git = "https://github.com/Hactchubas/zombie_scape" }
```

## Related

- [zombie_scape](https://github.com/Hactchubas/zombie_scape) - The core library with all game logic, AI, pathfinding, and steering behaviors. This server is just a thin wrapper that exposes the library over WebSocket.
