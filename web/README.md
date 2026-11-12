# Rust WebAssembly target

The browser build uses the existing Rust `ChessApp` and engine. There is no maintained JavaScript, HTML, or CSS frontend source in this directory.

## Build

From the repository root:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --locked
sh scripts/fetch-pieces.sh
cargo build --release --lib --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir web/pkg target/wasm32-unknown-unknown/release/chess_engine.wasm
```

The generated `web/pkg/` files are build artifacts. The Rust WebAssembly entrypoint creates its canvas from the browser document, so the app does not depend on a checked-in HTML page. A hosting environment still needs a small page or server response to load the generated WebAssembly binding module.

The WebAssembly target currently reuses the native Rust chess UI and local engine. Online room-code play still requires connecting the Rust client UI to the multiplayer server; the previous JavaScript client has been removed rather than retained.
