#![allow(unused_must_use)]

use std::io::{Cursor, Read};
use std::time::Duration;
use std::{collections::BTreeMap, error::Error, fs::File, path::Path};

use actix_cors::Cors;
use actix_web::{http::StatusCode, web, App, HttpResponse, HttpResponseBuilder, HttpServer};
use curiosity::{db::Db, serialization_crimes, CuriosityError, CuriosityResult, Season, SeasonId};

use curiosity::serialization_crimes::*;

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use smartstring::{Compact, SmartString};
use zip::read::ZipFile;

#[derive(Serialize, Deserialize)]
struct SearchRequest {
    #[serde(alias = "q", default)]
    query: SmartString<Compact>,
    #[serde(default)]
    kind: QueryKind,
    #[serde(default, deserialize_with = "deserialize_stringified_list")]
    seasons: SmallVec<[SeasonId; 16]>,
    #[serde(default)]
    highlight: bool,
    #[serde(default)]
    _curiosity_internal_offset: usize,
    #[serde(default)]
    page: Option<String>,
    #[serde(default = "page_size_default")]
    page_size: usize,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "kebab-case")]
enum QueryKind {
    #[default]
    Keywords,
    Phrase,
    Web,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "kebab-case")]
enum ResponseSerializationKind {
    #[default]
    Json,
    Msgpack,
}

#[derive(Serialize)]
struct Response<'a> {
    next_page: Option<String>,
    episodes: serialization_crimes::ResultsSerializer<'a>,
}

fn do_search(
    query: SearchRequest,
    db: web::Data<Db>,
    ser_kind: ResponseSerializationKind,
) -> CuriosityResult<HttpResponse> {
    let mut query = if let Some(page) = query.page.as_ref().filter(|page| page.as_str() != "null") {
        let mut out = Vec::with_capacity(128);
        base64_url::decode_to_vec(&page, &mut out);
        rmp_serde::from_slice(&out)?
    } else {
        query
    };

    let (parsed_query, is_phrase_query) = match query.kind {
        QueryKind::Phrase if query.query.split_ascii_whitespace().take(2).count() >= 2 => {
            (db.phrase_query(&query.query).boxed(), true)
        }
        QueryKind::Web => (db.parse_query(&query.query)?.boxed(), false),
        _ => (db.keyword_query(&query.query).boxed(), false),
    };

    let page_size = std::cmp::min(100, query.page_size);

    let results = db.search(
        &parsed_query,
        query.seasons.clone(),
        page_size,
        query._curiosity_internal_offset,
    )?;
    let next_page = if results.len() >= page_size {
        query._curiosity_internal_offset += results.len();
        Some(base64_url::encode(&rmp_serde::to_vec(&query)?))
    } else {
        None
    };

    let res = Response {
        next_page,
        episodes: serialization_crimes::ResultsSerializer::new(
            serialization_crimes::ResultsSerializerState {
                query: parsed_query,
                highlight: query.highlight,
                is_phrase_query,
                results,
                store: &db.store,
            },
        ),
    };

    Ok(match ser_kind {
        ResponseSerializationKind::Json => HttpResponseBuilder::new(StatusCode::OK).json(res),
        ResponseSerializationKind::Msgpack => HttpResponseBuilder::new(StatusCode::OK)
            .content_type("application/msgpack")
            .body(rmp_serde::to_vec_named(&res)?),
    })
}

#[actix_web::get("/search")]
async fn search(
    query: web::Query<SearchRequest>,
    db: web::Data<Db>,
) -> CuriosityResult<HttpResponse> {
    do_search(query.into_inner(), db, ResponseSerializationKind::Json)
}

#[actix_web::get("/search/{ser_kind}")]
async fn search_with_custom_serialization(
    query: web::Query<SearchRequest>,
    db: web::Data<Db>,
    ser_kind: web::Path<ResponseSerializationKind>,
) -> CuriosityResult<HttpResponse> {
    do_search(query.into_inner(), db, ser_kind.into_inner())
}

async fn update_database_periodically(db: Db) -> CuriosityResult<()> {
    let mut interval = actix_web::rt::time::interval(Duration::from_secs(6 * 60 * 60));
    loop {
        interval.tick().await;
        println!("Updating!");

        if let Err(e) = update_database(db.clone()).await {
            println!("error during db update: {e}");
        }
    }

    Ok(())
}

async fn update_database(mut db: Db) -> CuriosityResult<()> {
    let mirror_bytes = reqwest::get("https://github.com/emily-signet/transcripts-at-the-table-mirror/archive/refs/heads/data.zip").await?.bytes().await?;

    actix_web::rt::task::spawn_blocking(move || {
        let mut mirror = zip::ZipArchive::new(Cursor::new(mirror_bytes.as_ref()))
            .map_err(|e| CuriosityError::AnyhowError(e.into()))?;
        let seasons: BTreeMap<SeasonId, Season> = serde_json::from_reader(
            mirror
                .by_name("transcripts-at-the-table-mirror-data/seasons.json")
                .map_err(|e| CuriosityError::AnyhowError(e.into()))?,
        )?;

        db.add_documents(seasons.values(), |_, episode| {
            println!("reading {}", episode.title);
            let mut f = mirror
                .by_name(
                    (Path::new("transcripts-at-the-table-mirror-data/")
                        .join(episode.download.as_ref().unwrap().plain.clone()))
                    .to_str()
                    .unwrap(),
                )
                .map_err(|e| CuriosityError::AnyhowError(e.into()))?;

            let mut out = String::with_capacity((f.compressed_size() * 2) as usize);
            f.read_to_string(&mut out)?;
            Ok(out)
        })?;

        Ok(())
    })
    .await
    .unwrap()
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    std::fs::remove_file("./store.redb");
    std::fs::create_dir_all("./satt_idx");
    let db = Db::new("./satt_idx", "./store.redb")?;

    update_database(db.clone()).await.unwrap();

    let db_for_update = db.clone();
    actix_web::rt::spawn(update_database_periodically(db_for_update));

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .service(
                web::scope("/api")
                    .service(search)
                    .service(search_with_custom_serialization),
            )
            .service(actix_files::Files::new("/", "./static").index_file("index.html"))
            .app_data(web::Data::new(db.clone()))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await?;

    Ok(())
}
// fn main() {}
