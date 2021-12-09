use crate::song::{Song, SongResult};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

const LIBRARY_FILE: &str = "library.json";

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

    pub fn scan(directory: &Path) -> Result<Self, std::io::Error> {
        fn scan_dir(dir: &Path, records: &mut HashMap<u64, Song>) -> Result<(), std::io::Error> {
            let dir_entries = std::fs::read_dir(dir)?;
            for entry in dir_entries {
                let path = entry?.path();
                if path.is_dir() {
                    scan_dir(&path, records)?;
                } else if let Some(s) = path.to_str() {
                    if let Ok(s) = Song::new(s) {
                        records.insert(s.id, s);
                    }
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

    pub fn query(&self, terms: SearchTerms) -> SearchResults {
        let search_terms = terms.clone();
        let SearchTerms {
            artist,
            album,
            term,
            limit,
            sort_by,
            after,
        } = terms;
        let limit = limit.unwrap_or(SearchTerms::DEFAULT_LIMIT) as usize;
        let artist = artist.unwrap_or_else(String::new).to_lowercase();
        let album = album.unwrap_or_else(String::new).to_lowercase();
        let term = term.unwrap_or_else(String::new).to_lowercase();
        let sort_by = sort_by.unwrap_or(SortBy::track);

        let mut results: Box<dyn Iterator<Item = _>> = Box::new(self.records.values());

        if !artist.is_empty() {
            results = Box::new(results.filter(|song| song.artist_lower == artist));
        }

        if !album.is_empty() {
            results = Box::new(results.filter(|song| song.album_lower == album));
        }

        if !term.is_empty() {
            results = Box::new(results.filter(|song| {
                song.title_lower.contains(&term[..])
                    || song.artist_lower.contains(&term[..])
                    || song.album_lower.contains(&term[..])
                    || song.stem_lower.contains(&term[..])
            }));
        }

        // Sorting results: First, _everything_ is sorted. By default, it'll be by title.
        // If there's an `after` (ie, we've paginated to next), we will know how to filter before sorting
        if let Some(after) = after {
            if let Some(after) = self.records.get(&after) {
                // Keep only those records that are > `after`, depending on the filtering scheme
                results = Box::new(
                    results.filter(|song| song.cmp(after, sort_by) == std::cmp::Ordering::Greater),
                );
            }
        }

        // After filtering, we can sort:
        let mut results = results.collect::<Vec<_>>();
        results.sort_unstable_by(|&a, &b| a.cmp(b, sort_by));

        SearchResults {
            has_more: results.len() > limit,
            search_terms,
            results: results.into_iter().take(limit).map(|s| s.into()).collect(),
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

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[allow(non_camel_case_types)]
pub enum SortBy {
    title,
    artist,
    album,
    duration,
    track,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchTerms {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub term: Option<String>,
    pub limit: Option<u16>,
    //pub fuzzy: Option<u8>,
    pub sort_by: Option<SortBy>,
    pub after: Option<u64>,
}

#[derive(Serialize)]
pub struct SearchResults {
    has_more: bool,
    search_terms: SearchTerms,
    results: Vec<SongResult>,
}

impl SearchTerms {
    const DEFAULT_LIMIT: u16 = 100;
}

pub(crate) fn load_db(directories: Vec<PathBuf>) -> Option<MusicDB> {
    if directories.is_empty() {
        // Nothing to scan - just load the library file if possible.
        let start = std::time::Instant::now();
        if let Ok(db) = MusicDB::from_file(LIBRARY_FILE) {
            println!(
                "Loaded {} files from {} in {:.2?}",
                db.records.len(),
                LIBRARY_FILE,
                start.elapsed()
            );

            Some(db)
        } else {
            eprintln!(
                "No directories were specified for scanning, and no {} is present.",
                LIBRARY_FILE
            );
            eprintln!("Start this server with --scan=path/to/directory to scan for music.");
            None
        }
    } else {
        println!("Scanning for MP3s...");
        let start = std::time::Instant::now();
        let scanned = directories
            .iter()
            .filter_map(|dir| MusicDB::scan(dir).ok())
            .fold(MusicDB::default(), |a, b| a + b);

        let elapsed = start.elapsed();
        println!("Scanned {} files in {:.2?}", scanned.records.len(), elapsed);

        let db = if let Ok(existing) = MusicDB::from_file(LIBRARY_FILE) {
            existing + scanned
        } else {
            scanned
        };

        db.save_to(LIBRARY_FILE).ok();

        Some(db)
    }
}
