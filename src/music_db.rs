use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

use serde::Deserialize;

use crate::song::{Song, SongResult};

#[derive(Default)]
pub(crate) struct MusicDB {
    pub records: HashMap<u64, Song>,
}

impl MusicDB {
    pub fn from_file(filename: &str) -> Result<Self, std::io::Error> {
        let file = File::open(filename)?;
        let buf = BufReader::new(file);
        let records = buf
            .lines()
            .filter_map(|line| line.ok())
            .filter_map(|line| serde_json::from_str::<Song>(&line).ok())
            // Check that the song referenced exists
            .filter(|song| Path::new(&song.path).exists())
            .map(|s| (s.id, s))
            .collect();

        Ok(Self { records })
    }

    pub fn scan(directory: &str) -> Result<Self, std::io::Error> {
        fn scan_dir(dir: &str, records: &mut HashMap<u64, Song>) -> Result<(), std::io::Error> {
            let dir_entries = std::fs::read_dir(dir)?;
            for entry in dir_entries {
                let path = entry?.path();
                if path.is_dir() {
                    scan_dir(path.to_str().unwrap(), records)?;
                } else if let Ok(s) = Song::new(path.to_str().unwrap()) {
                    records.insert(s.id, s);
                }
            }
            Ok(())
        }

        let mut records = HashMap::new();
        scan_dir(directory, &mut records)?;

        Ok(Self { records })
    }

    pub fn save_to(&self, filename: &str) -> Result<(), std::io::Error> {
        let file = File::create(filename)?;
        let mut buf = BufWriter::new(file);

        for song in self.records.values() {
            if let Ok(s) = serde_json::to_string(&song) {
                writeln!(buf, "{}", s)?;
            }
        }

        Ok(())
    }

    pub fn query(&self, terms: &SearchTerms) -> Vec<SongResult> {
        let mut results: Box<dyn Iterator<Item = _>> = Box::new(self.records.values());

        if let Some(artist) = &terms.artist {
            results = Box::new(results.filter(|song| song.artist == *artist));
        }

        if let Some(album) = &terms.album {
            results = Box::new(results.filter(|song| song.album == *album));
        }

        if let Some(term) = &terms.term {
            let term = term.to_lowercase();
            results
                .filter(|song| {
                    song.title.to_lowercase().contains(&term[..])
                        || song.artist.to_lowercase().contains(&term[..])
                        || song.album.to_lowercase().contains(&term[..])
                })
                .map(|s| s.into())
                .collect()
        } else {
            results.map(|s| s.into()).collect()
        }
    }
}

impl std::ops::Add for MusicDB {
    type Output = MusicDB;

    fn add(self, rhs: Self) -> Self::Output {
        let MusicDB { mut records } = self;

        records.extend(rhs.records);

        MusicDB { records }
    }
}

#[derive(Deserialize, Debug)]
pub struct SearchTerms {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub term: Option<String>,
}
