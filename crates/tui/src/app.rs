use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;
use tokio::sync::mpsc::UnboundedSender;
use ytaudio_core::{
    config::Config,
    player::{Player, PlayerEvent},
    queue::Queue,
    source::{Track, Ytdlp},
};

pub enum AppEvent {
    Key(KeyEvent),
    Resize,
    Tick,
    Player(PlayerEvent),
    SearchDone(anyhow::Result<Vec<Track>>),
}

#[derive(PartialEq, Clone, Copy)]
pub enum Screen {
    Search,
    Queue,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Mode {
    Normal,
    Typing,
}

pub struct App {
    pub screen: Screen,
    pub mode: Mode,
    pub input: String,
    pub results: Vec<Track>,
    pub results_state: ListState,
    pub queue: Queue,
    pub queue_state: ListState,
    pub status: String,
    pub loading: bool,
    pub now: Option<Track>,
    pub position: f64,
    pub duration: f64,
    pub paused: bool,
    pub volume: f64,
    pub should_quit: bool,

    player: Player,
    source: Ytdlp,
    tx: UnboundedSender<AppEvent>,
    search_limit: usize,
}

impl App {
    pub fn new(player: Player, source: Ytdlp, tx: UnboundedSender<AppEvent>, cfg: &Config) -> App {
        let volume = cfg.volume.min(100) as f64;
        let _ = player.set_volume(volume);
        App {
            screen: Screen::Search,
            mode: Mode::Normal,
            input: String::new(),
            results: vec![],
            results_state: ListState::default(),
            queue: Queue::default(),
            queue_state: ListState::default(),
            status: String::new(),
            loading: false,
            now: None,
            position: 0.0,
            duration: 0.0,
            paused: false,
            volume,
            should_quit: false,
            player,
            source,
            tx,
            search_limit: cfg.search_limit,
        }
    }

    /// Handles one event. Returns true if the screen should be redrawn.
    pub fn handle(&mut self, ev: AppEvent) -> bool {
        match ev {
            AppEvent::Key(k) => {
                self.on_key(k);
                true
            }
            AppEvent::Resize => true,
            // Only redraw on ticks while something is moving on screen.
            AppEvent::Tick => self.loading || (self.now.is_some() && !self.paused),
            AppEvent::Player(p) => self.on_player(p),
            AppEvent::SearchDone(result) => {
                self.loading = false;
                match result {
                    Ok(tracks) => {
                        self.status = format!("{} results", tracks.len());
                        self.results_state
                            .select(if tracks.is_empty() { None } else { Some(0) });
                        self.results = tracks;
                    }
                    Err(e) => self.status = format!("Error: {e}"),
                }
                true
            }
        }
    }

    fn on_player(&mut self, ev: PlayerEvent) -> bool {
        match ev {
            // These arrive many times a second; don't redraw for them.
            PlayerEvent::Position(p) => {
                self.position = p;
                false
            }
            PlayerEvent::Duration(d) => {
                self.duration = d;
                true
            }
            PlayerEvent::Paused(p) => {
                self.paused = p;
                true
            }
            PlayerEvent::Volume(v) => {
                self.volume = v;
                true
            }
            PlayerEvent::TrackEnded => {
                self.next_track(true);
                true
            }
            PlayerEvent::Error(e) => {
                self.status = format!("Error: {e}");
                true
            }
        }
    }

    fn on_key(&mut self, k: KeyEvent) {
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        if self.mode == Mode::Typing {
            match k.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Enter => {
                    self.mode = Mode::Normal;
                    self.search();
                }
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Char(c) => self.input.push(c),
                _ => {}
            }
            return;
        }

        match k.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('1') => self.screen = Screen::Search,
            KeyCode::Char('2') => self.screen = Screen::Queue,
            KeyCode::Char('/') => {
                self.screen = Screen::Search;
                self.mode = Mode::Typing;
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Enter => self.on_enter(),
            KeyCode::Char('a') => {
                if self.screen == Screen::Search {
                    if let Some(track) = self.selected_result() {
                        self.add_to_queue(track);
                        self.status = "Added to queue".into();
                    }
                }
            }
            KeyCode::Char('d') => self.remove_selected_from_queue(),
            KeyCode::Char(' ') => {
                let _ = self.player.toggle_pause();
            }
            KeyCode::Char('n') => self.next_track(false),
            KeyCode::Char('p') => {
                if let Some(track) = self.queue.prev() {
                    self.start(track);
                }
            }
            KeyCode::Left => {
                let _ = self.player.seek(-5.0);
            }
            KeyCode::Right => {
                let _ = self.player.seek(5.0);
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                let _ = self.player.set_volume((self.volume + 5.0).min(100.0));
            }
            KeyCode::Char('-') => {
                let _ = self.player.set_volume((self.volume - 5.0).max(0.0));
            }
            KeyCode::Char('r') => {
                self.queue.repeat = self.queue.repeat.cycle();
            }
            _ => {}
        }
    }

    fn on_enter(&mut self) {
        match self.screen {
            Screen::Search => {
                if let Some(track) = self.selected_result() {
                    if track.is_playlist() {
                        // Open the playlist: its songs replace the results list.
                        self.input = track.url();
                        self.search();
                    } else {
                        let idx = self.add_to_queue(track);
                        self.play_index(idx);
                    }
                }
            }
            Screen::Queue => {
                if let Some(i) = self.queue_state.selected() {
                    self.play_index(i);
                }
            }
        }
    }

    fn selected_result(&self) -> Option<Track> {
        self.results.get(self.results_state.selected()?).cloned()
    }

    fn add_to_queue(&mut self, track: Track) -> usize {
        let idx = self.queue.add(track);
        if self.queue_state.selected().is_none() {
            self.queue_state.select(Some(0));
        }
        idx
    }

    fn remove_selected_from_queue(&mut self) {
        if self.screen != Screen::Queue {
            return;
        }
        if let Some(i) = self.queue_state.selected() {
            self.queue.remove(i);
            let len = self.queue.items.len();
            self.queue_state.select(if len == 0 {
                None
            } else {
                Some(i.min(len - 1))
            });
        }
    }

    fn move_selection(&mut self, delta: i32) {
        let (state, len) = match self.screen {
            Screen::Search => (&mut self.results_state, self.results.len()),
            Screen::Queue => (&mut self.queue_state, self.queue.items.len()),
        };
        if len == 0 {
            return;
        }
        let current = state.selected().unwrap_or(0) as i32;
        let next = (current + delta).clamp(0, len as i32 - 1) as usize;
        state.select(Some(next));
    }

    fn search(&mut self) {
        let query = self.input.trim().to_string();
        if query.is_empty() {
            return;
        }
        self.loading = true;
        self.status = format!("Loading \"{query}\"...");
        let source = self.source.clone();
        let tx = self.tx.clone();
        let limit = self.search_limit;
        tokio::spawn(async move {
            let result = source.resolve(&query, limit).await;
            let _ = tx.send(AppEvent::SearchDone(result));
        });
    }

    fn play_index(&mut self, idx: usize) {
        if let Some(track) = self.queue.play(idx) {
            self.start(track);
        }
    }

    fn next_track(&mut self, auto: bool) {
        match self.queue.advance(auto) {
            Some(track) => self.start(track),
            None if auto => {
                self.now = None;
                self.position = 0.0;
                self.duration = 0.0;
                self.status = "Queue finished".into();
            }
            None => self.status = "End of queue".into(),
        }
    }

    fn start(&mut self, track: Track) {
        if let Err(e) = self.player.load(&track.url()) {
            self.status = format!("Error: {e}");
            return;
        }
        self.position = 0.0;
        self.duration = track.duration.unwrap_or(0.0);
        self.status.clear();
        self.now = Some(track);
    }
}