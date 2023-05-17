#![allow(unused_must_use)]

use std::io::{Cursor, Read};

use std::sync::Arc;
use std::time::Duration;
use std::{collections::BTreeMap, error::Error, path::Path};

use actix_cors::Cors;
use actix_web::{http::StatusCode, web, App, HttpResponse, HttpResponseBuilder, HttpServer};

use curiosity::store::TermsToSentencesId;
use curiosity::{db::Db, CuriosityError, CuriosityResult, Season, SeasonId};

use curiosity::{serialization_crimes::*, StoredEpisode};

use nyoom_json::{Serializer, UnescapedStr};

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use smartstring::{Compact, SmartString};
use tinyset::SetU32;

macro_rules! noescape {
    ($l:expr) => {
        // aesthetics
        UnescapedStr::create($l)
    };
}

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

#[actix_web::get("/search")]
async fn search(
    query: web::Query<SearchRequest>,
    db: web::Data<Db>,
) -> CuriosityResult<HttpResponse> {
    let query = query.into_inner();

    let mut query = if let Some(page) = query.page.as_ref().filter(|page| page.as_str() != "null") {
        let mut out = Vec::with_capacity(128);
        base64_url::decode_to_vec(&page, &mut out);
        postcard::from_bytes(&out)?
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

    // let page_size = 1;
    let page_size = std::cmp::min(100, query.page_size);

    let results = db.search(
        &parsed_query,
        query.seasons.clone(),
        page_size,
        query._curiosity_internal_offset,
    )?;
    let next_page = if results.len() >= page_size {
        query._curiosity_internal_offset += results.len();
        Some(base64_url::encode(&postcard::to_stdvec(&query)?))
    } else {
        None
    };

    let mut out = String::with_capacity(50_000);
    let mut ser = Serializer::new(&mut out);
    let mut response_obj = ser.object();
    response_obj.field(UnescapedStr::create("next_page"), next_page.as_deref());

    let txn = db.store.begin_read()?;
    let ep_db = db.store.get_docs_accessor(txn.get())?;

    let mut episodes = response_obj.array_field("episodes");
    for (doc_id, _) in results {
        let doc_bytes = ep_db.get_doc(doc_id.0)?;
        let doc = unsafe { rkyv::archived_root::<StoredEpisode>(doc_bytes.value()) };

        let mut episode = episodes.add_object();
        episode.field(noescape!("curiosity_id"), doc.id.value());
        episode.field(noescape!("slug"), doc.slug.as_str());
        episode.field(noescape!("title"), doc.title.as_str());
        if let Some(docs_id) = doc.docs_id.as_ref() {
            episode.field("docs_id", docs_id.as_str());
        }

        episode.field("season", noescape!(doc.season.as_ref()));

        if !query.highlight {
            episode.end();
            continue;
        }

        let mut term_to_sentence_id = TermsToSentencesId::new(doc_id.0, 0);

        let mut highlights = episode.array_field(noescape!("highlights"));
        let mut seen_sentences: SetU32 = SetU32::new();

        for term in parsed_query.terms.iter() {
            term_to_sentence_id.set_term(*term);
            let Some(sentence_ids) = ep_db.get_sentences(&term_to_sentence_id)? else {
                continue;
            };

            for sentence_id in sentence_ids.value().ids.iter() {
                if !seen_sentences.insert(sentence_id.value()) {
                    continue;
                }

                let sentence = &doc.tokens[sentence_id.value() as usize];
                if let Some(highlighted) =
                    sentence.highlight(&parsed_query.terms, &doc.text, is_phrase_query)
                {
                    highlighted.serialize_into(highlights.add_array());
                }
            }
        }

        highlights.end();

        episode.end();
    }

    episodes.end();

    response_obj.end();

    Ok(HttpResponseBuilder::new(StatusCode::OK)
        .content_type("application/json")
        .body(out))
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
}

async fn update_database(db: Db) -> CuriosityResult<()> {
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
    let mut db = Db::new("./satt_idx", "./store.redb")?;
    update_database(db.clone()).await.unwrap();

    Arc::<redb::Database>::get_mut(&mut db.store.db)
        .unwrap()
        .compact();

    let db_for_update = db.clone();
    actix_web::rt::spawn(update_database_periodically(db_for_update));

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .service(web::scope("/api").service(search))
            .service(actix_files::Files::new("/", "./static").index_file("index.html"))
            .app_data(web::Data::new(db.clone()))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await?;

    Ok(())
}
// fn main() {}
