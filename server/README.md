# Chess Engine multiplayer server

Standalone Rust WebSocket server for Chess Engine global chat, match chat, and room-code multiplayer. It is separate from the Android and Windows app builds.

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

- Global chat: `{"type":"chat","scope":"global","name":"Player","message":"Hello everyone"}`
- Match chat: `{"type":"chat","scope":"match","name":"Player","message":"Good luck"}`

Global messages are broadcast to every connected client. Match messages are sent only to the two clients in the same room. Names are limited to 24 characters and messages to 500 characters.

## Current scope

The shared Rust app now includes a WebSocket chat panel with Global and Match tabs, a server-address field, display name, and room create/join controls. The app does not ship with a hosted server address: deploy this server and enter its WebSocket URL ending in `/ws` in the app. Match chat requires creating or joining a room. Rooms are held in memory and are lost when the server restarts. Use a persistent store and add reconnection/authentication/rate limits before running a public production service.
