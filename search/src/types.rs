use super::highlighter;
use actix_web::{error::ResponseError, HttpResponse};
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

lazy_static::lazy_static! {
    static ref WEBSEARCH_REGEX: Regex = Regex::new("OR|-").unwrap();
}

const fn default_highlights_per_entry() -> usize {
    4
}

#[derive(Deserialize)]
pub(crate) struct SearchQuery {
    pub(crate) season: String,
    pub(crate) q: String,
    #[serde(default)]
    pub(crate) query_fmt: SearchFunctions,
    #[serde(default)]
    pub(crate) return_text: bool,
    #[serde(default)]
    pub(crate) return_highlights: bool,
    #[serde(default = "default_highlights_per_entry")]
    pub(crate) highlights_per_entry: usize,
}

#[derive(Serialize)]
pub(crate) struct SearchResult {
    pub(crate) title: String,
    pub(crate) id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rank: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) highlights: Option<Vec<String>>,
}

#[derive(Serialize)]
pub(crate) struct SearchResponse {
    pub(crate) status: &'static str,
    pub(crate) data: Vec<SearchResult>,
}

#[derive(Serialize)]
pub(crate) struct ErrResponse {
    pub(crate) status: &'static str,
    pub(crate) kind: &'static str,
    pub(crate) why: String,
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SearchFunctions {
    Plain,
    Phrase,
    WebSearch,
    Exact,
}

impl Default for SearchFunctions {
    fn default() -> Self {
        SearchFunctions::WebSearch
    }
}

impl SearchFunctions {
    pub(crate) fn to_pg_function(&self) -> &'static str {
        match self {
            SearchFunctions::Plain => "plainto_tsquery",
            SearchFunctions::Phrase => "phraseto_tsquery",
            SearchFunctions::WebSearch => "websearch_to_tsquery",
            _ => "intentionally_erroring_function",
        }
    }

    pub(crate) fn keywords_to_regex(&self, text: &str) -> Vec<Regex> {
        match self {
            // ideally i'd like this to not be an owning thingy, but since the others involve replace it's a bit more convenient
            SearchFunctions::Plain => text
                .split_ascii_whitespace()
                .map(highlighter::input_to_regex)
                .collect(),
            SearchFunctions::Phrase | SearchFunctions::Exact => {
                vec![highlighter::input_to_regex(text)]
            }
            SearchFunctions::WebSearch => WEBSEARCH_REGEX
                .replace_all(text, "")
                .split_ascii_whitespace()
                .map(highlighter::input_to_regex)
                .collect(),
        }
    }
}

#[derive(Error, Debug)]
pub(crate) enum SearchError {
    #[error(transparent)]
    PoolError(#[from] deadpool_postgres::PoolError),
    #[error(transparent)]
    PostgresError(#[from] tokio_postgres::Error),
    #[error("id or season not found")]
    NotFound,
}

impl ResponseError for SearchError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        use SearchError::*;
        let response_body = ErrResponse {
            status: "err",
            kind: match self {
                PoolError(_) => "db pool error",
                PostgresError(_) => "postgres error",
                NotFound => "object not found",
            },
            why: format!("{}", self),
        };

        match self {
            NotFound => HttpResponse::NotFound(),
            _ => HttpResponse::InternalServerError(),
        }
        .json(response_body)
    }
}
