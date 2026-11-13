# Chess Engine multiplayer server

Single Rust server for the Chess Engine browser app, global chat, match chat, and room-code multiplayer. Android and Windows builds connect to this same server.

## Run

```sh
cd server
cargo run --release
```

The server listens on `0.0.0.0:8080` by default. Set `BIND_ADDR` to override it. The release binary embeds the Rust WebAssembly browser app, so the same process serves the website and multiplayer API. Deploy behind FreeSrv's HTTPS/WSS proxy.

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

The native and browser apps use the same Rust chess UI. The server build workflow first compiles the browser app to WebAssembly, generates its loader, and embeds both files into the Linux server binary. Upload that one binary to FreeSrv; it serves the website at `/` and multiplayer at `/ws`. Client WebSocket URL: `wss://chessengine.freesrv.app/ws`. Match chat requires creating or joining a room. Rooms are held in memory and are lost when the server restarts. Use a persistent store and add reconnection/authentication/rate limits before running a public production service.
