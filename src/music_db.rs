use crate::song::{Song, SongResult};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
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
    pub fn new(filename: &str) -> Self {
        match Self::from_file(filename) {
            Ok(s) => s,
            Err(_) => Self {
                records: HashMap::new(),
            },
        }
    }

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

    /// Scans `directory` for music.
    ///
    /// If `rescan_files` is set, individual MP3 files will be rescanned (parsing their ID3 tags,
    /// for example); if false, then they will be parsed only if they aren't already in the database.
    ///
    /// A note on perf:
    /// On a moderate (~4000 file) input, avoiding the rescan drops load time from about 7m to 1m.
    ///
    /// Keeping track of the known files as a HashSet instead of searching `self.records` further
    /// drops the time from 1m to 30s.
    fn scan_directory(
        &mut self,
        known_files: &mut HashSet<String>,
        directory: &Path,
        rescan_files: bool,
    ) -> Result<(), std::io::Error> {
        // Recursively search a directory
        for entry in std::fs::read_dir(directory)?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                self.scan_directory(known_files, &path, rescan_files)?;
            } else if let Some(s) = path.to_str() {
                if !rescan_files && known_files.contains(s) {
                    //if !rescan_files && self.contains_file(s) {
                    // no need to scan this file
                } else if let Ok(s) = Song::new(s) {
                    known_files.insert(s.path.clone());
                    self.records.insert(s.id, s);
                }
            }
        }

        Ok(())
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

    pub fn query(&self, search_terms: SearchTerms) -> SearchResults {
        let SearchTerms {
            artist,
            album,
            term,
            limit,
            sort_by,
            after,
        } = search_terms.clone();

        let limit = limit.unwrap_or(SearchTerms::DEFAULT_LIMIT) as usize;
        let artist = artist.unwrap_or_default().to_lowercase();
        let album = album.unwrap_or_default().to_lowercase();
        let term = term.unwrap_or_default().to_lowercase();
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

        // After filtering, we can sort and take the first n:
        let mut results = results.collect::<Vec<_>>();
        results.sort_unstable_by(|&a, &b| a.cmp(b, sort_by));
        let results = results
            .into_iter()
            .take(limit)
            .map(|s| s.into())
            .collect::<Vec<_>>();

        let other_albums = if !artist.is_empty() {
            // Find all albums by this artist
            let artist_lower = artist.to_lowercase();
            Some(
                self.records
                    .values()
                    .filter(|&s| s.artist_lower == artist_lower)
                    .map(|s| s.album.clone())
                    .collect(),
            )
        } else if !album.is_empty() {
            // Find all artists associated with this album name
            let album_lower = album.to_lowercase();
            let artists = self
                .records
                .values()
                .filter(|&s| s.album_lower == album_lower)
                .map(|s| s.artist.to_lowercase())
                .collect::<HashSet<_>>();

            // Then all albums for these artists except the one specified
            Some(
                self.records
                    .values()
                    .filter(|&s| *s.album_lower != album_lower)
                    .filter(|&s| artists.contains(&s.artist_lower))
                    .map(|s| s.album.clone())
                    .collect(),
            )
        } else {
            None
        };

        SearchResults {
            has_more: results.len() > limit,
            search_terms,
            results,
            other_albums,
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
    pub sort_by: Option<SortBy>,
    pub after: Option<u64>,
}

#[derive(Serialize)]
pub struct SearchResults {
    has_more: bool,
    search_terms: SearchTerms,
    results: Vec<SongResult>,

    other_albums: Option<HashSet<String>>,
}

impl SearchTerms {
    const DEFAULT_LIMIT: u16 = 100;
}

pub(crate) fn load_db(directories: Vec<(PathBuf, bool)>) -> Option<MusicDB> {
    if directories.is_empty() {
        // Nothing to scan - just load the library file if possible.
        let start = std::time::Instant::now();
        if let Ok(db) = MusicDB::from_file(LIBRARY_FILE) {
            println!(
                "Loaded {} files from {LIBRARY_FILE} in {:.2?}",
                db.records.len(),
                start.elapsed()
            );

            Some(db)
        } else {
            eprintln!(
                "No directories were specified for scanning, and {LIBRARY_FILE} wasn't present."
            );
            eprintln!("Start this server with --scan=path/to/directory or --rescan=path/to/directory to scan for music.");
            None
        }
    } else {
        println!("Scanning for MP3s...");
        let start = std::time::Instant::now();
        let mut db = MusicDB::new(LIBRARY_FILE);

        let mut known_files = db.records.values().map(|s| s.path.to_string()).collect();

        for (directory, rescan_files) in directories {
            db.scan_directory(&mut known_files, &directory, rescan_files)
                .ok();
        }

        let elapsed = start.elapsed();
        println!("Scanned {} files in {:.2?}", db.records.len(), elapsed);

        db.save_to(LIBRARY_FILE).ok();

        Some(db)
    }
}
