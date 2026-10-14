use anyhow::{Result, anyhow};
use directories::ProjectDirs;
use serde::Deserialize;
use std::path::PathBuf;

const DEFAULT_CONFIG: &str = r#"# ytaudio configuration

# Program locations. Leave as-is if mpv and yt-dlp are on your PATH.
mpv_path = "mpv"
ytdlp_path = "yt-dlp"

# Login. Use ONE of these two lines (remove the leading #), or neither.
# cookies_from_browser = "brave"
cookies_file = 'C:\Users\Duggu\cookies.txt'

volume = 70
search_limit = 15
"#;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub mpv_path: String,
    pub ytdlp_path: String,
    pub cookies_from_browser: Option<String>,
    pub cookies_file: Option<String>,
    pub volume: u8,
    pub search_limit: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            mpv_path: "mpv".into(),
            ytdlp_path: "yt-dlp".into(),
            cookies_from_browser: None,
            cookies_file: None,
            volume: 70,
            search_limit: 15,
        }
    }
}

impl Config {
    /// Where the config file lives (on Windows: %APPDATA%\ytaudio\config\config.toml).
    pub fn path() -> PathBuf {
        ProjectDirs::from("", "", "ytaudio")
            .map(|d| d.config_dir().join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    }

    /// Loads the config, creating a commented default file on first run.
    pub fn load() -> Result<Config> {
        let path = Self::path();
        if !path.exists() {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&path, DEFAULT_CONFIG)?;
        }
        let text = std::fs::read_to_string(&path)?;
        toml::from_str(&text).map_err(|e| anyhow!("bad config file {}: {e}", path.display()))
    }

    /// Extra arguments for yt-dlp so it uses your login.
    pub fn ytdlp_cookie_args(&self) -> Vec<String> {
        if let Some(file) = &self.cookies_file {
            vec!["--cookies".into(), file.clone()]
        } else if let Some(browser) = &self.cookies_from_browser {
            vec!["--cookies-from-browser".into(), browser.clone()]
        } else {
            vec![]
        }
    }

    /// Extra arguments for mpv, which runs yt-dlp itself when it plays a YouTube URL.
    pub fn mpv_cookie_args(&self) -> Vec<String> {
        if let Some(file) = &self.cookies_file {
            // %N% = length-prefixed value, so backslashes and spaces in the path are safe.
            vec![format!(
                "--ytdl-raw-options=cookies=%{}%{}",
                file.len(),
                file
            )]
        } else if let Some(browser) = &self.cookies_from_browser {
            vec![format!("--ytdl-raw-options=cookies-from-browser={browser}")]
        } else {
            vec![]
        }
    }
}
