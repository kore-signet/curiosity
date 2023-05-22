use actix_web::{body::BoxBody, HttpResponseBuilder, ResponseError};
use curiosity::CuriosityError;
use reqwest::StatusCode;
use thiserror::Error;

pub mod api;
pub mod update;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error(transparent)]
    CuriosityLibError(#[from] CuriosityError),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    PostcardError(#[from] postcard::Error),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    ZipError(#[from] zip::result::ZipError),
    #[error(transparent)]
    REDBError(#[from] redb::Error),
    #[error("invalid page token")]
    BadPageToken,
}

impl ResponseError for ServerError {
    fn error_response(&self) -> actix_web::HttpResponse<BoxBody> {
        use ServerError::*;

        let mut status = StatusCode::INTERNAL_SERVER_ERROR;
        let (err_kind, err_msg) = match self {
            CuriosityLibError(e) => {
                use CuriosityError::*;
                match e {
                    QueryParserError(e) => {
                        status = StatusCode::BAD_REQUEST;
                        ("query", e.to_string())
                    }
                    Tantivy(e) => ("internal", e.to_string()),
                    TantivyOpenError(e) => ("internal", e.to_string()),
                    IOError(e) => ("internal", e.to_string()),
                    REDBError(e) => ("internal", e.to_string()),
                    PostcardError(e) => ("internal", e.to_string()),
                    NotFound => ("internal", "document not found".to_string()),
                }
            }
            REDBError(e) => ("internal", e.to_string()),
            IOError(e) => ("internal", e.to_string()),
            SerdeJsonError(e) => ("internal", e.to_string()),
            PostcardError(e) => ("internal", e.to_string()),
            ReqwestError(e) => ("internal", e.to_string()),
            ZipError(e) => ("internal", e.to_string()),
            BadPageToken => {
                status = StatusCode::BAD_REQUEST;
                ("page", BadPageToken.to_string())
            }
        };

        #[derive(serde::Serialize)]
        struct ErrResponse<'a> {
            err: bool,
            kind: &'a str,
            msg: &'a str,
        }

        HttpResponseBuilder::new(status).json(ErrResponse {
            err: true,
            kind: err_kind,
            msg: &err_msg,
        })
    }
}

pub type ServerResult<T> = Result<T, ServerError>;
