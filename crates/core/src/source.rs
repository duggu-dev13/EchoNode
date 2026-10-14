#[derive(Debug, Clone, serde::Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    #[serde(alias = "uploader")]
    pub channel: Option<String>,
    pub duration: Option<f64>,
}

impl Track {
    pub fn url(&self) -> String {
        format!("https://www.youtube.com/watch?v={}", self.id)
    }
}