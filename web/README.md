# Chess Engine Web

A lightweight browser client for Chess Engine. It is designed to work on phones, tablets, and desktops without installing an app.

## Run locally

From the repository root, serve the `web/` directory with any static HTTP server, for example:

```sh
cd web
python -m http.server 8000
```

Then open `http://localhost:8000`.

The page imports chess.js from jsDelivr, so an internet connection is required for the chess rules library.

## Online multiplayer

The online panel speaks the room-code WebSocket protocol implemented by `../server`:

- Create: `{"type":"create"}`
- Join: `{"type":"join","code":"ABC123"}`
- Move: `{"type":"move","uci":"e2e4"}`

Set the WebSocket server field to the deployed server's `wss://.../ws` address. The repository's server is not deployed by this project, so the placeholder address must be replaced before online play works.

The server remains authoritative for move validation and broadcasts the resulting FEN to both players.

## Current scope

- Responsive browser board
- Local two-player games
- Online room creation and joining
- Server-synchronized moves and room codes

The web client is an initial frontend. The native Rust search engine is not yet compiled to WebAssembly, so browser AI play is not included yet. Promotion currently defaults to a queen in the web client.
