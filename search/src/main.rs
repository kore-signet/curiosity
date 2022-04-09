use actix_files::Files as StaticFiles;
use actix_web::{get, web, App, HttpServer};
use deadpool_postgres::{Pool as DbPool, Runtime};
use regex::{Regex, RegexSetBuilder};
use tokio_postgres::NoTls;

mod types;
use types::*;
pub(crate) mod highlighter;

lazy_static::lazy_static! {
    static ref POSTGRES_ILIKE_ESCAPER: Regex = Regex::new(r"=|%|_").unwrap();
}

#[derive(serde::Deserialize, serde::Serialize)]
struct Config {
    pg: deadpool_postgres::Config,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let mut cfg = config::Config::new();
        cfg.merge(config::Environment::new().separator("_"))
            .unwrap();
        cfg.try_into()
    }
}

#[get("/api/transcript/{episode_id}")]
async fn get_transcript(
    db: web::Data<DbPool>,
    episode_id: web::Path<String>,
) -> Result<String, SearchError> {
    let conn = db.get().await?;

    conn.query_opt(
        "SELECT body FROM episodes WHERE id = $1",
        &[episode_id.as_ref()],
    )
    .await?
    .and_then(|row| row.try_get::<usize, String>(0).ok())
    .ok_or(SearchError::NotFound)
}

#[get("/api/season/{season}")]
async fn get_season(
    db: web::Data<DbPool>,
    season: web::Path<String>,
) -> Result<web::Json<SearchResponse>, SearchError> {
    let conn = db.get().await?;

    let rows = conn
        .query(
            "SELECT title, id FROM episodes WHERE season = $1",
            &[season.as_ref()],
        )
        .await?;

    Ok(web::Json(SearchResponse {
        status: "ok",
        data: rows
            .into_iter()
            .map(|row| SearchResult {
                title: row.get::<&str, String>("title"),
                id: row.get::<&str, String>("id"),
                rank: None,
                body: None,
                highlights: None,
            })
            .collect(),
    }))
}

#[get("/api/search")]
async fn search(
    db: web::Data<DbPool>,
    query: web::Query<SearchQuery>,
) -> Result<web::Json<SearchResponse>, SearchError> {
    let conn = db.get().await?;

    let query_function = query.query_fmt.to_pg_function();

    let (keyword_regexes, keyword_set) = if query.return_highlights {
        let mut keywords = query.query_fmt.keywords_to_regex(&query.q);

        if query.query_fmt != SearchFunctions::Exact {
            // going through postgres here is not the most efficient but it simplifies things
            let stemmed_keywords_row = conn
                .query_one(
                    "SELECT tsvector_to_array(to_tsvector('fatt',$1)) AS stems",
                    &[&query.q],
                )
                .await?;

            keywords.extend(
                stemmed_keywords_row
                    .get::<&str, Vec<&str>>("stems")
                    .into_iter()
                    .map(highlighter::input_to_regex),
            );
        }

        let underlying_regexes = keywords.iter().map(|v| v.as_str()).collect::<Vec<&str>>();

        let set = RegexSetBuilder::new(underlying_regexes)
            .case_insensitive(true)
            .build()
            .unwrap();

        (Some(keywords), Some(set))
    } else {
        (None, None)
    };

    let res = if query.query_fmt == SearchFunctions::Exact {
        // escape with =
        let escaped_query = POSTGRES_ILIKE_ESCAPER.replace_all(&query.q, "=$0");

        // exact query path
        // parameters: season $1, escaped query $2
        conn.query(
            r#"
            SELECT
                title,
                body,
                id
            FROM episodes
            WHERE season = $1 AND body ILIKE ('%' || $2 || '%') ESCAPE '='
            "#,
            &[&query.season, &escaped_query],
        )
        .await?
    } else {
        // full text query path
        // parameters: season $1, query $2, headline settings $3
        conn.query(
            &format!(
                r#"
                SELECT 
                    title,
                    body,
                    id,
                    ts_rank_cd(body_index, query) AS rank
                FROM episodes, {query_function}('fatt', $2) query
                WHERE season = $1 AND query @@ body_index
                ORDER BY rank DESC
            "#
            ),
            &[&query.season, &query.q],
        )
        .await?
    };

    Ok(web::Json(SearchResponse {
        status: "ok",
        data: res
            .into_iter()
            .map(|row| SearchResult {
                title: row.get::<&str, String>("title"),
                id: row.get::<&str, String>("id"),
                rank: row.try_get::<&str, f32>("rank").ok(),
                body: if query.return_text {
                    Some(row.get::<&str, String>("body"))
                } else {
                    None
                },
                highlights: if query.return_highlights {
                    Some(highlighter::highlights(
                        row.get::<&str, &str>("body"),
                        r#"<highlight>$0</highlight>"#,
                        keyword_regexes.as_ref().unwrap(),
                        keyword_set.as_ref().unwrap(),
                        query.highlights_per_entry,
                    ))
                } else {
                    None
                },
            })
            .collect(),
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cfg = Config::from_env().unwrap();
    let pool = cfg.pg.create_pool(Some(Runtime::Tokio1), NoTls).unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(search)
            .service(get_season)
            .service(get_transcript)
            .service(StaticFiles::new("/", "./static").index_file("index.html"))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
