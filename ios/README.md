# iOS build preparation

The app identifier is `com.chess.engine`, matching Android.

This directory is intentionally not connected to GitHub Actions. No iOS workflow is added or triggered.

## Requirements

A signed iOS app package (`.ipa`) requires Apple's iOS SDK and code-signing tools. The final archive/export step must run on macOS with Xcode and an Apple signing identity. A Mac can be rented or used through a cloud Mac service; owning one is not required.

## Rust targets

On a Mac with the Rust toolchain installed:

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
```

The device target is `aarch64-apple-ios`. The simulator target is `aarch64-apple-ios-sim`.

## Current status

This repository has the shared Rust application code and the stable bundle identifier prepared. It does not yet contain an Xcode project, signing configuration, provisioning profile, or IPA export workflow. Those are required before an installable IPA can be produced.

Do not commit Apple certificates, private keys, or provisioning profiles to this repository.
