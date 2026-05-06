# RapidChecksum

A Linux clone of [RapidCRC Unicode](https://www.ov2.eu/programs/rapidcrc-unicode) built with Rust and QT (via [cxx-qt](https://github.com/KDAB/cxx-qt)).

## Features

- Hash algorithms: CRC32, MD5, SHA1, SHA256, SHA512, ED2K, BLAKE3, BLAKE2sp, SHA3-224, SHA3-256, SHA3-384, SHA3-512
- Export results as SFV (CRC32) or standard `hash *filename` format (all other algorithms)

## Dependencies

- Rust (see `rust-toolchain.toml` for pinned version)
- QT 6 with QtQuick, QtQuick.Controls, and Qt Widgets
- KDE Frameworks: `qt6-declarative`, `kf6-qqc2-desktop-style` (for `org.kde.desktop` style)

## Build

```sh
./build.sh
```

The binary is written to `target/release/rapidchecksum`.

## Flatpak

```sh
./build-flatpak.sh
```

## Architecture

| Layer | Tech |
|-------|------|
| UI | Qt Widgets (`src/qt_app.cpp`) |
| Qt/Rust bridge | cxx-qt 0.8 (`src/app_backend.rs`) |
| Hashing | Pure Rust (`src/hasher/`) - crc32fast, md-5, sha1, sha2, sha3, ed2k, blake3, blake2s_simd |
| File I/O | `src/fileio.rs` - SFV / hash file read & write |
| Worker | `src/worker.rs` - background thread with progress channel |
| Config | `src/config.rs` - persisted settings via serde\_json |
