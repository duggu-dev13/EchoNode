use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::ClientOptions;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Position(f64),
    Duration(f64),
    Paused(bool),
    Volume(f64),
    /// The track finished by itself (not skipped or stopped).
    TrackEnded,
    Error(String),
}

pub struct Player {
    cmd_tx: mpsc::UnboundedSender<Value>,
    // Kept alive so mpv is killed automatically when Player is dropped.
    _child: Child,
}

impl Player {
    /// Starts mpv in the background (no window, no video) and connects to it.
    pub async fn spawn(
        mpv_path: &str,
        extra_args: Vec<String>,
    ) -> Result<(Player, mpsc::UnboundedReceiver<PlayerEvent>)> {
        let pipe_name = format!(r"\\.\pipe\ytaudio-{}", std::process::id());

        let child = Command::new(mpv_path)
            .args([
                "--no-video",
                "--idle=yes",
                "--no-terminal",
                "--force-window=no",
                "--ytdl-format=bestaudio/best",
            ])
            .arg(format!("--input-ipc-server={pipe_name}"))
            .args(&extra_args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                anyhow!("could not start mpv (\"{mpv_path}\"): {e}. Is it installed and on PATH?")
            })?;

        // mpv needs a moment to create its control pipe.
        let mut pipe = None;
        for _ in 0..50 {
            match ClientOptions::new().open(&pipe_name) {
                Ok(p) => {
                    pipe = Some(p);
                    break;
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
            }
        }
        let pipe = pipe.ok_or_else(|| anyhow!("mpv started but its control pipe never appeared"))?;
        let (reader, mut writer) = tokio::io::split(pipe);

        // Writer task: sends queued commands to mpv, one JSON object per line.
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<Value>();
        tokio::spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                let line = format!("{cmd}\n");
                if writer.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
            }
        });

        // Reader task: turns mpv's JSON lines into PlayerEvents.
        let (ev_tx, ev_rx) = mpsc::unbounded_channel::<PlayerEvent>();
        tokio::spawn(async move {
            let mut lines = BufReader::new(reader).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(v) = serde_json::from_str::<Value>(&line) {
                    if let Some(ev) = map_event(&v) {
                        if ev_tx.send(ev).is_err() {
                            return;
                        }
                    }
                }
            }
            let _ = ev_tx.send(PlayerEvent::Error("mpv closed the connection".into()));
        });

        let player = Player {
            cmd_tx,
            _child: child,
        };

        // Ask mpv to tell us when these change.
        for (id, prop) in [(1, "time-pos"), (2, "duration"), (3, "pause"), (4, "volume")] {
            player.send(json!({ "command": ["observe_property", id, prop] }))?;
        }

        Ok((player, ev_rx))
    }

    fn send(&self, cmd: Value) -> Result<()> {
        self.cmd_tx
            .send(cmd)
            .map_err(|_| anyhow!("lost connection to mpv"))
    }

    /// Plays a URL immediately (replacing whatever is playing) and unpauses.
    pub fn load(&self, url: &str) -> Result<()> {
        self.send(json!({ "command": ["loadfile", url, "replace"] }))?;
        self.send(json!({ "command": ["set_property", "pause", false] }))
    }

    pub fn toggle_pause(&self) -> Result<()> {
        self.send(json!({ "command": ["cycle", "pause"] }))
    }

    /// Seeks relative to the current position (negative = backwards).
    pub fn seek(&self, secs: f64) -> Result<()> {
        self.send(json!({ "command": ["seek", secs, "relative"] }))
    }

    pub fn set_volume(&self, volume: f64) -> Result<()> {
        self.send(json!({ "command": ["set_property", "volume", volume] }))
    }

    pub fn stop(&self) -> Result<()> {
        self.send(json!({ "command": ["stop"] }))
    }
}

fn map_event(v: &Value) -> Option<PlayerEvent> {
    match v.get("event")?.as_str()? {
        "property-change" => {
            let name = v.get("name")?.as_str()?;
            let data = v.get("data")?;
            match name {
                "time-pos" => data.as_f64().map(PlayerEvent::Position),
                "duration" => data.as_f64().map(PlayerEvent::Duration),
                "pause" => data.as_bool().map(PlayerEvent::Paused),
                "volume" => data.as_f64().map(PlayerEvent::Volume),
                _ => None,
            }
        }
        "end-file" => match v.get("reason").and_then(|r| r.as_str()) {
            Some("eof") => Some(PlayerEvent::TrackEnded),
            Some("error") => Some(PlayerEvent::Error(
                v.get("file_error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("playback error")
                    .to_string(),
            )),
            _ => None,
        },
        _ => None,
    }
}