# EchoNano v1.0.0

A lightweight, audio-only YouTube player for the terminal. It streams only the
audio, so it uses far less CPU and RAM than playing music in a browser tab. 

> For personal use. ytaudio (_BuildName) only streams audio; it has no download feature.

## Latest release
- [EchoNano](https://jpmasmntqklatmaksado.supabase.co/storage/v1/object/public/Portfolio-Media/src/EchoNano_release/ytaudio_v1.0.0.exe)

## Requirements
- Windows 10/11
- [mpv](https://mpv.io) and [yt-dlp](https://github.com/yt-dlp/yt-dlp) on your PATH

## Run
    cargo run --release -p EchoNano

## Keys
| Key | Action |
|---|---|
| `/` | Search (or paste a playlist URL) |
| `Enter` | Play / open playlist |
| `a` | Add to queue |
| `1` / `2` | Search screen / Queue screen |
| `Space` | Pause / resume |
| `n` / `p` | Next / previous |
| `<` `>` arrows | Seek 5 seconds |
| `+` / `-` | Volume |
| `r` | Cycle repeat mode |
| `d` | Remove from queue |
| `q` | Quit |


## Contributing
See CONTRIBUTING.md.