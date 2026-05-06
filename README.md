# RapidChecksum

A Linux clone of [RapidCRC Unicode](https://www.ov2.eu/programs/rapidcrc-unicode) built with Rust and QT (via [cxx-qt](https://github.com/KDAB/cxx-qt)).

## Features

- Hash algorithms: CRC32, MD5, SHA1, SHA256, SHA512, ED2K, BLAKE3, BLAKE2sp, SHA3-224, SHA3-256, SHA3-384, SHA3-512
- Load UTF-8 SFV files for CRC32 verification without auto-starting hashing
- Export UTF-8 CRC32 results as SFV or standard `hash *filename` format for other algorithms
- Flatpak builds install FreeDesktop metadata for `.sfv` file association

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

### KDE Dolphin context menu integration

After installing the flatpak, run the following command once to add a **RapidChecksum** entry to Dolphin's right-click context menu:

```sh
flatpak run --command=install-dolphin-servicemenu io.github.l_raider.rapidchecksum
```

Then restart Dolphin. Right-clicking any file will show a **RapidChecksum** entry that opens the selected files in the app without auto-starting hashing.

## Known Issues

### Slow file loading when selecting many files from Dolphin

When right-clicking a large number of files (25+) in Dolphin and choosing RapidChecksum,
there is a noticeable delay before the app opens. This is caused by two separate issues:

**1. KDE Activity Manager (primary bottleneck — [KDE Bug #440766](https://bugs.kde.org/show_bug.cgi?id=440766))**

Before Dolphin launches any service menu action, it records every selected file in the
recent documents history via `kactivitymanagerd`. This is done one file at a time,
synchronously, before your action even starts. On KDE Frameworks 6 this is ~3× slower
than KF5. With 500 files the delay can be 10–30+ seconds.

This is a KDE bug and is not fixable from RapidChecksum's side. To work around it,
disable recent document tracking in KDE System Settings:

> **System Settings → Workspace → Recent Files → uncheck "Keep history of opened documents"**

Alternatively, stop the service for the current session:
```sh
systemctl --user stop plasma-kactivitymanagerd.service
```

**2. Flatpak document portal overhead (secondary)**

This has been fixed in the RapidChecksum service menu: the app uses direct file paths
instead of portal file forwarding, since the flatpak already has `--filesystem=home`
access.

## Architecture

| Layer | Tech |
|-------|------|
| UI | Qt Widgets (`src/qt_app.cpp`) |
| Qt/Rust bridge | cxx-qt 0.8 (`src/app_backend.rs`) |
| Hashing | Pure Rust (`src/hasher/`) - crc32fast, md-5, sha1, sha2, sha3, ed2k, blake3, blake2s_simd |
| File I/O | `src/fileio.rs` - UTF-8 SFV read & write |
| Worker | `src/worker.rs` - background thread with progress channel |
| Config | `src/config.rs` - persisted settings via serde\_json |
