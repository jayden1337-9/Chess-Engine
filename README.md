# Chess Engine

Chess Engine is a cross-platform chess project built in Rust.

## Project targets

- **Android:** Native Rust/egui application.
- **WebAssembly:** The same Rust/egui application compiled to WebAssembly. Browser UI logic is Rust; no hand-written JavaScript, HTML, or CSS frontend is maintained.
- **Server:** Rust WebSocket service in `server/` for room-code multiplayer.

## WebAssembly build

The `wasm32-unknown-unknown` target is built by the **WebAssembly Build** GitHub Actions workflow. It compiles the Rust app and generates the browser bindings as build output. See [web/README.md](web/README.md) for details.

The release server binary embeds the generated WebAssembly app and serves it at `https://chessengine.freesrv.com/`; the same server provides multiplayer at `wss://chessengine.freesrv.com/ws`.

## Rust engine

The engine includes legal move generation, material and piece-activity evaluation, iterative-deepening alpha-beta search, quiescence search, and move ordering.

Run tests and the command-line UCI engine with:

```sh
cargo test
cargo run --release
```

## Android

Android is the first native application target. Build instructions and workflows are in `.github/workflows/`.

## Multiplayer server

See [server/README.md](server/README.md). The server hosts the browser app and multiplayer service together at `chessengine.freesrv.com`.

## Piece artwork

The CBurnett PNG assets are fetched with `scripts/fetch-pieces.sh`. See [asset attribution](assets/pieces/ATTRIBUTION.md) for source and license.
