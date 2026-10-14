use crate::source::Track;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Repeat {
    #[default]
    Off,
    One,
    All,
}

impl Repeat {
    pub fn cycle(self) -> Self {
        match self {
            Repeat::Off => Repeat::All,
            Repeat::All => Repeat::One,
            Repeat::One => Repeat::Off,
        }
    }
}

#[derive(Debug, Default)]
pub struct Queue {
    pub items: Vec<Track>,
    pub current: Option<usize>,
    pub repeat: Repeat,
}

impl Queue {
    /// Adds a track (no duplicates) and returns its index.
    pub fn add(&mut self, track: Track) -> usize {
        if let Some(i) = self.items.iter().position(|t| t.id == track.id) {
            return i;
        }
        self.items.push(track);
        self.items.len() - 1
    }

    /// Marks `idx` as the current track and returns it.
    pub fn play(&mut self, idx: usize) -> Option<Track> {
        let track = self.items.get(idx)?.clone();
        self.current = Some(idx);
        Some(track)
    }

    /// Moves to the next track. `auto` is true when the previous one ended by itself.
    pub fn advance(&mut self, auto: bool) -> Option<Track> {
        if self.items.is_empty() {
            return None;
        }
        let next = match self.current {
            None => Some(0),
            Some(i) if auto && self.repeat == Repeat::One => Some(i),
            Some(i) if i + 1 < self.items.len() => Some(i + 1),
            Some(_) if self.repeat == Repeat::All => Some(0),
            Some(_) => None,
        };
        match next {
            Some(i) => self.play(i),
            None => {
                if auto {
                    self.current = None;
                }
                None
            }
        }
    }

    pub fn prev(&mut self) -> Option<Track> {
        let i = self.current?;
        self.play(i.saturating_sub(1))
    }

    pub fn remove(&mut self, idx: usize) {
        if idx >= self.items.len() {
            return;
        }
        self.items.remove(idx);
        self.current = match self.current {
            Some(c) if idx < c => Some(c - 1),
            Some(c) if idx == c => c.checked_sub(1),
            other => other,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: &str) -> Track {
        Track {
            id: id.into(),
            ..Default::default()
        }
    }

    #[test]
    fn advances_then_stops() {
        let mut q = Queue::default();
        q.add(t("a"));
        q.add(t("b"));
        assert_eq!(q.play(0).unwrap().id, "a");
        assert_eq!(q.advance(true).unwrap().id, "b");
        assert!(q.advance(true).is_none());
        assert_eq!(q.current, None);
    }

    #[test]
    fn repeat_all_wraps_around() {
        let mut q = Queue::default();
        q.repeat = Repeat::All;
        q.add(t("a"));
        q.add(t("b"));
        q.play(1);
        assert_eq!(q.advance(true).unwrap().id, "a");
    }
}
