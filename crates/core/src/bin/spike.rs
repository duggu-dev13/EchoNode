use std::time::Duration;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::windows::named_pipe::ClientOptions,
    process::Command,
};

const PIPE: &str = r"\\.\pipe\ytaudio-mpv";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::args().nth(1).expect("usage: spike <url>");

    let mut child = Command::new("mpv")
        .args([
            "--no-video",
            "--idle=yes",
            "--no-terminal",
            "--ytdl-format=bestaudio",
            &format!("--input-ipc-server={PIPE}"),
        ])
        .kill_on_drop(true)
        .spawn()?;

    // mpv needs a moment to create the pipe
    let mut pipe = None;
    for _ in 0..50 {
        if let Ok(p) = ClientOptions::new().open(PIPE) {
            pipe = Some(p);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let pipe = pipe.ok_or_else(|| anyhow::anyhow!("mpv pipe never appeared"))?;

    let (rd, mut wr) = tokio::io::split(pipe);
    let cmd = serde_json::json!({ "command": ["loadfile", url] });
    wr.write_all(format!("{cmd}\n").as_bytes()).await?;

    let mut lines = BufReader::new(rd).lines();
    while let Some(line) = lines.next_line().await? {
        println!("{line}");
    }
    child.wait().await?;
    Ok(())
}
