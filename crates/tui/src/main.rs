mod app;
mod ui;

use anyhow::Result;
use app::{App, AppEvent};
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use std::time::Duration;
use tokio::sync::mpsc;
use ytaudio_core::{config::Config, player::Player, source::Ytdlp};

#[tokio::main]
async fn main() -> Result<()> {
    // `ytaudio --config-path` prints where the config file is.
    if std::env::args().nth(1).as_deref() == Some("--config-path") {
        println!("{}", Config::path().display());
        return Ok(());
    }

    let cfg = Config::load()?;

    // Start mpv BEFORE taking over the terminal, so startup errors print normally.
    let (player, mut player_rx) = Player::spawn(&cfg.mpv_path, cfg.mpv_cookie_args()).await?;
    let source = Ytdlp {
        path: cfg.ytdlp_path.clone(),
        cookie_args: cfg.ytdlp_cookie_args(),
    };

    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    // 1) Keyboard: a plain thread, because reading keys blocks.
    let key_tx = tx.clone();
    std::thread::spawn(move || {
        loop {
            match event::read() {
                Ok(Event::Key(k)) if k.kind == KeyEventKind::Press => {
                    if key_tx.send(AppEvent::Key(k)).is_err() {
                        break;
                    }
                }
                Ok(Event::Resize(_, _)) => {
                    if key_tx.send(AppEvent::Resize).is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });

    // 2) A slow timer that refreshes the progress bar (4 times per second).
    let tick_tx = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(250));
        loop {
            interval.tick().await;
            if tick_tx.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });

    // 3) Forward mpv events into the same channel.
    let player_tx = tx.clone();
    tokio::spawn(async move {
        while let Some(ev) = player_rx.recv().await {
            if player_tx.send(AppEvent::Player(ev)).is_err() {
                break;
            }
        }
    });

    let mut app = App::new(player, source, tx, &cfg);

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app, &mut rx).await;
    ratatui::restore();
    result
}

async fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    rx: &mut mpsc::UnboundedReceiver<AppEvent>,
) -> Result<()> {
    terminal.draw(|f| ui::draw(f, app))?;
    while let Some(ev) = rx.recv().await {
        let redraw = app.handle(ev);
        if app.should_quit {
            break;
        }
        if redraw {
            terminal.draw(|f| ui::draw(f, app))?;
        }
    }
    Ok(())
}
