# Chess Engine

Chess Engine is a cross-platform chess project with a Rust engine, Android app, and lightweight browser client.

## Project targets

- **Android:** Native Rust/egui application.
- **Web:** Responsive browser client in `web/`; no installation required.
- **Server:** Rust WebSocket service in `server/` for room-code multiplayer.

## Web version

See [web/README.md](web/README.md) for local setup and multiplayer configuration. The web client currently supports local two-player games and online room-code play. Browser AI powered by the Rust engine is planned, not yet included.

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

See [server/README.md](server/README.md). The server must be deployed separately and exposed over TLS before online play is available.

## Piece artwork

The CBurnett PNG assets are fetched with `scripts/fetch-pieces.sh`. See [asset attribution](assets/pieces/ATTRIBUTION.md) for source and license.
