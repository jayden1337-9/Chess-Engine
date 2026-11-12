# Chess Engine multiplayer server

Standalone WebSocket server for room-code games. It is intentionally separate from the Android/iOS app build and is not part of the existing GitHub Actions workflows.

## Run

```sh
cd server
cargo run --release
```

The server listens on `0.0.0.0:8080` by default. Set `BIND_ADDR` to override it. Deploy behind a TLS reverse proxy and connect clients using `wss://`.

- Health: `GET /health`
- WebSocket: `/ws`

## Initial JSON protocol

Send one JSON message per WebSocket text frame:

- Create: `{"type":"create"}`
- Join: `{"type":"join","code":"7KQ9PX"}`
- Move: `{"type":"move","uci":"e2e4"}`
- Resign: `{"type":"resign"}`
- Offer draw: `{"type":"draw_offer"}`

The server returns `created` or `joined` with a randomly generated six-character room code, player color and initial FEN. Legal moves are checked against the authoritative server-side chess position and broadcast to both players.

## Current scope

This is the multiplayer server foundation. The desktop/mobile app's online-room screens and WebSocket client still need to be connected to this protocol. Rooms are held in memory and are lost when the server restarts. Use a persistent store and add reconnection/authentication/rate limits before running a public production service.
