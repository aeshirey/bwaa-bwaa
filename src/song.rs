use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::time::Duration;

#[derive(Debug, Hash, Default, Serialize, Deserialize)]
pub struct Song {
    pub id: u64,
    pub path: String,
    pub title: String,

    pub artist: String,
    pub album: String,
    pub year: u16,
    pub comment: String,
    //pub genre: Genre,
    pub duration: Duration,
}

impl Song {
    pub fn new(filename: &str) -> Result<Self, std::io::Error> {
        // For now, only mp3s are supported:
        let mut song = Self::from_mp3(filename).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Can't read MP3 metadata")
        })?;

        let mut hasher = DefaultHasher::new();
        song.hash(&mut hasher);
        song.id = hasher.finish();

        Ok(song)
    }

    fn from_mp3(filename: &str) -> Option<Song> {
        let metadata = mp3_metadata::read_from_file(filename).ok()?;

        let song = if metadata.optional_info.is_empty() {
            let tags = metadata.tag?;

            Song {
                path: filename.to_string(),
                title: tags.title,
                duration: metadata.duration,
                ..Default::default()
            }
        } else {
            let info = metadata.optional_info.into_iter().next()?;
            Song {
                path: filename.to_string(),
                title: info.title.unwrap_or_else(String::new),
                artist: if info.performers.is_empty() {
                    "".to_string()
                } else {
                    info.performers[0].to_string()
                },
                album: info.album_movie_show.unwrap_or_else(String::new),
                duration: metadata.duration,
                ..Default::default()
            }
        };

        Some(song)
    }

    pub fn duration_formatted(&self) -> String {
        let mut formatted = String::new();

        let mut s = self.duration.as_secs();

        let sec = s % 60;
        s /= 60;

        let min = s % 60;
        s /= 60;

        let hour = s % 24;

        if hour > 0 {
            formatted.push_str(&format!("{}:", hour));
        }

        formatted.push_str(&format!("{:02}:", min));
        formatted.push_str(&format!("{:02}", sec));

        formatted
    }

    pub fn file_stem(&self) -> Option<&str> {
        let stem = std::path::Path::new(&self.path).file_stem()?;
        stem.to_str()
    }
}

impl Display for Song {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, r#"<a href="{}">{}</a>"#, self.path, self.title)
    }
}

/// Used for sending search results to the client.
///
/// Note that this differs from `Song` in three ways:
/// * `path` is omitted for security
/// * `duration` is a string for easy display
/// * `id` is converted to a string because JS can't handle 64-bit integers
#[derive(Serialize)]
pub struct SongResult {
    pub id: String,
    pub title: String,

    pub artist: String,
    pub album: String,
    pub year: u16,
    pub comment: String,
    pub duration: String,
}

impl From<&Song> for SongResult {
    fn from(song: &Song) -> Self {
        let title = if song.title.is_empty() {
            match song.file_stem() {
                Some(s) => s.to_string(),
                None => "(unknown)".to_string(),
            }
        } else {
            song.title.clone()
        };

        SongResult {
            id: song.id.to_string(),
            title,
            artist: song.artist.clone(),
            album: song.album.clone(),
            year: song.year,
            comment: song.comment.clone(),
            duration: song.duration_formatted(),
        }
    }
}
