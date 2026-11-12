# Chess Engine

A Rust chess engine and Android chess app, with a Windows build planned after Android is complete.

## Current development

- Rust engine core
- Legal move generation through the `chess` crate
- Material and piece-activity evaluation
- Iterative-deepening alpha-beta search
- Quiescence search for tactical positions
- Capture, promotion, and check-aware move ordering
- Offline AI opponent with six strength settings
- UCI command-line interface
- CBurnett PNG piece assets

## Build the engine

```sh
cargo test
cargo run --release
```

The command-line build speaks UCI on standard input and output.

## Piece artwork

The CBurnett PNGs are fetched with `scripts/fetch-pieces.sh`. See [asset attribution](assets/pieces/ATTRIBUTION.md) for the source and license.

## Targets

Android is the first application target. iOS project preparation is documented in `ios/README.md`; iOS builds are intentionally not part of GitHub Actions. Windows x64 follows once the Android app is complete.


## Game modes

- Play vs AI
- Local two-player mode on one device

Online room-code multiplayer is being developed separately; the server protocol and deployment notes will live under `server/`.

## iOS

See [iOS build preparation](ios/README.md). No iOS workflow is configured or triggered by this repository.
