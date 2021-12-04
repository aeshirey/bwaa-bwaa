use askama::Template;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use warp::{http::Response, Filter};

mod music_db;
use music_db::{MusicDB, SearchTerms};
mod search;
use search::SearchResults;
mod song;

/// BWAA-BWAA! WHAT'S NEW, PUSSYCAT?
/// https://www.youtube.com/watch?v=Mw7Gryt-rcc
const WHATS_NEW_PUSSYCAT: &[u8; 28797] = include_bytes!("../What's new pussycat.mp3");

#[tokio::main]
async fn main() {
    let database = load_db().expect("Must pass --dir=<dir> or --library=<file>");
    let database = Arc::new(Mutex::new(database));
    let database = warp::any().map(move || Arc::clone(&database));

    let library = warp::path::end()
        .and(database.clone())
        .and_then(handle_library);

    let listen = warp::path!("listen")
        .and(warp::query().map(|map: HashMap<String, String>| map.get("id").unwrap().to_string()))
        .and(database.clone())
        .and_then(handle_listen);

    let search = warp::path!("search")
        .and(warp::query())
        .and(database.clone())
        .and_then(handle_search);

    let whats_new = warp::path!("whatsnew").and_then(handle_whats_new);

    let cors = warp::cors().allow_any_origin();

    let routes = library.or(listen).or(search).or(whats_new).with(cors);

    warp::serve(routes).run(([127, 0, 0, 1], 8001)).await;
}

fn load_db() -> Option<MusicDB> {
    let directories = std::env::args()
        .filter(|arg| arg.starts_with("--dir="))
        .map(|arg| arg.split_at("--dir=".len()).1.to_string())
        .collect::<Vec<String>>();

    let library = std::env::args()
        .find(|arg| arg.starts_with("--library="))
        .map(|arg| arg[10..].to_string());

    let start = std::time::Instant::now();

    let db = if directories.is_empty() {
        let library = library.expect("Must pass --dir=<dir> or --library=<file>");
        MusicDB::from_file(&library).ok()?
    } else {
        println!("Scanning for MP3s...");
        let db = directories
            .iter()
            .filter_map(|dir| MusicDB::scan(dir).ok())
            .fold(MusicDB::default(), |a, b| a + b);

        if let Some(library) = library {
            db.save_to(&library).ok();
        }

        db
    };

    let elapsed = start.elapsed();
    println!("Loaded {} files in {:.2?}", db.records.len(), elapsed,);

    Some(db)
}

async fn handle_library(
    database: Arc<Mutex<MusicDB>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let guard = database.lock().await;
    let results = guard.records.values().collect();

    let body = SearchResults { results }.render().unwrap();
    Ok(warp::reply::html(body))
}

async fn handle_listen(
    id: String,
    database: Arc<Mutex<MusicDB>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let db = database.lock().await;
    let id = id.parse::<u64>().unwrap();

    let song = match db.records.get(&id) {
        Some(s) => s,
        None => {
            let msg = format!("id={} not found", id);
            return Ok(Box::new(
                Response::builder()
                    .header("content-type", "text/plain")
                    .body(msg.into())
                    .unwrap(),
            ));
        }
    };

    let response = match std::fs::read(&song.path) {
        Ok(f) => Box::new(
            Response::builder()
                .header("content-type", "audio/mpeg")
                .body(f)
                .unwrap(),
        ),
        Err(e) => {
            eprintln!("Error with file {}: {:?}", song.path, e);
            let msg = format!("Unable to load file: {}", id);
            let b = msg.bytes().collect::<Vec<_>>();
            let _x = warp::reply::html(b);
            todo!()
        }
    };

    Ok(response)
}

async fn handle_search(
    terms: SearchTerms,
    database: Arc<Mutex<MusicDB>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let db = database.lock().await;
    let results = db.query(&terms);

    Ok(warp::reply::json(&results))
}

async fn handle_whats_new() -> Result<impl warp::Reply, warp::Rejection> {
    Ok(Response::builder()
        .header("content-type", "audio/mpeg")
        .body(WHATS_NEW_PUSSYCAT.to_vec())
        .unwrap())
}
