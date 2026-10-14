use crate::app::{App, Mode, Screen};
use ratatui::{
    prelude::*,
    widgets::{Block, Gauge, List, ListItem, Paragraph},
};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // search box
        Constraint::Min(3),    // list
        Constraint::Length(4), // now playing
        Constraint::Length(2), // status + key hints
    ])
    .split(f.area());

    // --- Search box with tab titles ---
    let tabs = match app.screen {
        Screen::Search => " [1] Search *   [2] Queue ",
        Screen::Queue => " [1] Search   [2] Queue * ",
    };
    let typing = app.mode == Mode::Typing;
    let text = if typing {
        format!("{}█", app.input)
    } else if app.input.is_empty() {
        "Press / to search (or paste a playlist URL, or type :ytrec / :ytsubs / :ythistory)"
            .to_string()
    } else {
        app.input.clone()
    };
    let style = if typing {
        Style::new().yellow()
    } else {
        Style::new()
    };
    f.render_widget(
        Paragraph::new(text)
            .style(style)
            .block(Block::bordered().title(tabs)),
        chunks[0],
    );

    // --- Main list ---
    match app.screen {
        Screen::Search => {
            let items: Vec<ListItem> = app
                .results
                .iter()
                .map(|t| {
                    ListItem::new(format!(
                        "{}  -  {}  [{}]",
                        t.title,
                        t.artist(),
                        t.duration_text()
                    ))
                })
                .collect();
            let list = List::new(items)
                .block(Block::bordered().title(" Results "))
                .highlight_style(Style::new().reversed())
                .highlight_symbol("> ");
            f.render_stateful_widget(list, chunks[1], &mut app.results_state);
        }
        Screen::Queue => {
            let current = app.queue.current;
            let items: Vec<ListItem> = app
                .queue
                .items
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let mark = if Some(i) == current { "♪ " } else { "  " };
                    ListItem::new(format!("{mark}{}  -  {}", t.title, t.artist()))
                })
                .collect();
            let list = List::new(items)
                .block(Block::bordered().title(" Queue "))
                .highlight_style(Style::new().reversed())
                .highlight_symbol("> ");
            f.render_stateful_widget(list, chunks[1], &mut app.queue_state);
        }
    }

    // --- Now playing ---
    let block = Block::bordered().title(" Now Playing ");
    let inner = block.inner(chunks[2]);
    f.render_widget(block, chunks[2]);
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(inner);

    let title = match &app.now {
        Some(t) => format!(
            "{} {}  -  {}",
            if app.paused { "||" } else { ">" },
            t.title,
            t.artist()
        ),
        None => "Nothing playing".to_string(),
    };
    f.render_widget(Paragraph::new(title), rows[0]);

    let ratio = if app.duration > 0.0 {
        (app.position / app.duration).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let label = format!(
        "{} / {}    vol {:.0}%    repeat: {:?}",
        fmt_time(app.position),
        fmt_time(app.duration),
        app.volume,
        app.queue.repeat
    );
    f.render_widget(Gauge::default().ratio(ratio).label(label), rows[1]);

    // --- Footer ---
    let footer = Paragraph::new(vec![
        Line::from(app.status.clone()).yellow(),
        Line::from(
            "/ search  Enter play  a add  Space pause  n/p next/prev  </> seek  +/- vol  r repeat  d delete  q quit",
        )
        .dark_gray(),
    ]);
    f.render_widget(footer, chunks[3]);
}

fn fmt_time(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}
