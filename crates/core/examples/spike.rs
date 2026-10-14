use std::time::Duration;
use tokio::time::sleep;
use ytaudio_core::player::{Player, PlayerEvent};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::args()
        .nth(1)
        .expect("usage: cargo run -p ytaudio-core --example spike -- <youtube-url>");

    let (player, mut events) = Player::spawn("mpv", vec![]).await?;
    player.load(&url)?;

    tokio::spawn(async move {
        let mut last_second = -1i64;
        while let Some(ev) = events.recv().await {
            match ev {
                PlayerEvent::Position(p) => {
                    if p as i64 != last_second {
                        last_second = p as i64;
                        println!("position: {last_second}s");
                    }
                }
                other => println!("event: {other:?}"),
            }
        }
    });

    sleep(Duration::from_secs(6)).await;
    println!(">> pause");
    player.toggle_pause()?;
    sleep(Duration::from_secs(2)).await;
    println!(">> resume");
    player.toggle_pause()?;
    println!(">> volume 40");
    player.set_volume(40.0)?;
    println!(">> seek +10s");
    player.seek(10.0)?;
    sleep(Duration::from_secs(6)).await;
    println!(">> done");
    Ok(())
}