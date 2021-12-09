use crate::song::SongResult;
use askama::Template;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
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

const FAVICON: &[u8; 15406] = include_bytes!("../favicon.ico");
const DEFAULT_PORT: u16 = 8081;

#[tokio::main]
async fn main() {
    let port = match std::env::var("PORT") {
        Ok(s) => s.parse().expect("Invalid port number specified"),
        Err(_) => DEFAULT_PORT,
    };

    let to_scan = std::env::args()
        .filter(|arg| arg.starts_with("--scan="))
        .map(|arg| PathBuf::from(&arg[7..]))
        .filter(|path| path.exists())
        .collect();
    let database = music_db::load_db(to_scan).expect("Failed to load database");
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

    let details = warp::path!("details")
        .and(warp::query().map(|map: HashMap<String, String>| map.get("id").unwrap().to_string()))
        .and(database.clone())
        .and_then(handle_details);

    let favicon = warp::path!("favicon.ico").map(|| {
        Response::builder()
            .header("content-type", "image/x-icon")
            .body(FAVICON.to_vec())
    });

    let whats_new = warp::path!("whatsnew").and_then(handle_whats_new);

    let cors = warp::cors().allow_any_origin();

    let routes = library
        .or(listen)
        .or(search)
        .or(whats_new)
        .or(details)
        .or(favicon)
        .with(cors);

    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
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

    if id == "whatsnew" {
        return Ok(Box::new(
            Response::builder()
                .header("content-type", "audio/mpeg")
                .body(WHATS_NEW_PUSSYCAT.to_vec())
                .unwrap(),
        ));
    }

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
    let results = db.query(terms);

    Ok(warp::reply::json(&results))
}

async fn handle_details(
    id: String,
    database: Arc<Mutex<MusicDB>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let db = database.lock().await;

    if id == "whatsnew" {
        let song = SongResult {
            id: "whatsnew".to_string(),
            title: "The best meal I've ever had in my life".to_string(),
            artist: "John Mulaney".to_string(),
            album: "Comedy Central Stand-Up".to_string(),
            year: 2019,
            comment: "https://www.youtube.com/watch?v=Mw7Gryt-rcc".to_string(),
            duration: "21 instances of \"What's New, Pussycat?\"".to_string(),
            track: None,
        };
        return Ok(warp::reply::json(&song));
    }

    let id = id.parse::<u64>().unwrap();
    match db.records.get(&id) {
        Some(s) => {
            let song: SongResult = s.into();
            Ok(warp::reply::json(&song))
        }
        None => Ok(warp::reply::json(&"?")),
    }
}

async fn handle_whats_new() -> Result<impl warp::Reply, warp::Rejection> {
    Ok(Response::builder()
        .header("content-type", "audio/mpeg")
        .body(WHATS_NEW_PUSSYCAT.to_vec())
        .unwrap())
}
