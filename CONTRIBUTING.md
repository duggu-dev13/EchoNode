# Contributing

1. Install Rust, mpv and yt-dlp.
2. `cargo test --workspace` must pass.
3. `cargo fmt --all` before committing.

Code layout:
- `crates/core`: all logic (yt-dlp, mpv, queue, config). No UI code here.
- `crates/tui`: the terminal interface.

Bug reports: include your Windows version, `mpv --version` and `yt-dlp --version`.