use anyhow::{Result, anyhow};
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Track {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub uploader: Option<String>,
    #[serde(default)]
    pub duration: Option<f64>,
}

impl Track {
    pub fn url(&self) -> String {
        match &self.url {
            Some(u) if !u.is_empty() => u.clone(),
            _ => format!("https://www.youtube.com/watch?v={}", self.id),
        }
    }

    pub fn artist(&self) -> &str {
        self.channel
            .as_deref()
            .or(self.uploader.as_deref())
            .unwrap_or("Unknown")
    }

    pub fn duration_text(&self) -> String {
        match self.duration {
            Some(d) => {
                let s = d as u64;
                format!("{}:{:02}", s / 60, s % 60)
            }
            None => "--:--".into(),
        }
    }

    /// Entries from "your playlists" pages point to playlists, not songs.
    pub fn is_playlist(&self) -> bool {
        self.url().contains("playlist?list=")
    }
}

#[derive(Debug, Clone)]
pub struct Ytdlp {
    pub path: String,
    pub cookie_args: Vec<String>,
}

impl Ytdlp {
    /// Search text -> YouTube search. A URL or a ":ytrec"-style shortcut -> list that page.
    pub async fn resolve(&self, input: &str, limit: usize) -> Result<Vec<Track>> {
        let input = input.trim();
        if input.starts_with("http") || input.starts_with(':') {
            self.run_flat(input).await
        } else {
            self.search(input, limit).await
        }
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<Track>> {
        self.run_flat(&format!("ytsearch{limit}:{query}")).await
    }

    async fn run_flat(&self, target: &str) -> Result<Vec<Track>> {
        let out = Command::new(&self.path)
            .args([
                "--flat-playlist",
                "--dump-json",
                "--no-warnings",
                "--ignore-errors",
            ])
            .args(&self.cookie_args)
            .arg(target)
            .stdin(Stdio::null())
            .output()
            .await
            .map_err(|e| {
                anyhow!(
                    "could not run yt-dlp (\"{}\"): {e}. Is it installed and on PATH?",
                    self.path
                )
            })?;

        let tracks: Vec<Track> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        if tracks.is_empty() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let last = stderr.lines().last().unwrap_or("no results");
            return Err(anyhow!("yt-dlp: {last}"));
        }
        Ok(tracks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flat_playlist_line() {
        let line = r#"{"_type":"url","id":"abc123","title":"Test Song","url":"https://www.youtube.com/watch?v=abc123","channel":"Some Channel","duration":215}"#;
        let t: Track = serde_json::from_str(line).unwrap();
        assert_eq!(t.title, "Test Song");
        assert_eq!(t.artist(), "Some Channel");
        assert_eq!(t.duration_text(), "3:35");
        assert!(!t.is_playlist());
    }
}
