use std::path::PathBuf;

pub mod db;
pub mod docs_accessor;
pub mod schema;
pub mod sentence;
pub mod serialization_crimes;
pub mod store;

#[cfg(feature = "actix")]
use actix_web::{body::BoxBody, http::StatusCode, HttpResponseBuilder, ResponseError};
use sentence::*;

use strum::{AsRefStr, Display, EnumString, FromRepr, IntoStaticStr};

use thiserror::Error;

#[derive(
    Debug,
    PartialEq,
    Eq,
    EnumString,
    AsRefStr,
    FromRepr,
    Display,
    IntoStaticStr,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Clone,
    Copy,
    serde::Serialize,
    serde::Deserialize,
    Ord,
    PartialOrd,
)]
#[serde(rename_all = "kebab-case")]
#[archive_attr(derive(
    Debug,
    PartialEq,
    Eq,
    EnumString,
    AsRefStr,
    FromRepr,
    Display,
    IntoStaticStr,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Clone,
    Copy,
    serde::Serialize,
    serde::Deserialize,
    Ord,
    PartialOrd,
))]
#[archive_attr(serde(rename_all = "kebab-case"))]
#[archive_attr(strum(serialize_all = "kebab-case"))]
#[repr(u64)]
pub enum SeasonId {
    #[strum(serialize = "autumn-in-hieron")]
    AutumnInHieron = 0,
    #[strum(serialize = "marielda")]
    Marielda = 1,
    #[strum(serialize = "winter-in-hieron")]
    WinterInHieron = 2,
    #[strum(serialize = "spring-in-hieron")]
    SpringInHieron = 3,
    #[strum(serialize = "counterweight")]
    Counterweight = 4,
    #[strum(serialize = "twilight-mirage")]
    TwilightMirage = 5,
    #[strum(serialize = "road-to-partizan")]
    RoadToPartizan = 6,
    #[strum(serialize = "partizan")]
    Partizan = 7,
    #[strum(serialize = "road-to-palisade")]
    RoadToPalisade = 8,
    #[strum(serialize = "palisade")]
    Palisade = 9,
    #[strum(serialize = "sangfielle")]
    Sangfielle = 10,
    #[strum(serialize = "extras")]
    Extras = 11,
    #[strum(serialize = "patreon")]
    Patreon = 12,
    #[strum(serialize = "unknown-string")]
    Other = 13,
}

#[derive(
    Debug,
    PartialEq,
    Eq,
    EnumString,
    AsRefStr,
    FromRepr,
    Display,
    IntoStaticStr,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Clone,
    Copy,
    serde::Serialize,
    serde::Deserialize,
    Ord,
    PartialOrd,
    Hash,
)]
#[archive_attr(derive(
    PartialEq,
    Eq,
    EnumString,
    AsRefStr,
    FromRepr,
    Display,
    IntoStaticStr,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Clone,
    Copy,
    serde::Serialize,
    serde::Deserialize,
    Ord,
    PartialOrd,
))]
#[archive_attr(serde(rename_all = "kebab-case"))]
#[serde(rename_all = "kebab-case")]
#[repr(u8)]
pub enum Friend {
    #[strum(serialize = "austin", serialize = "audtin", serialize = "austi")]
    Austin,
    #[strum(serialize = "jack")]
    Jack,
    #[strum(serialize = "sylvie", serialize = "sylvia", serialize = "sylvi")]
    Sylvi,
    #[strum(serialize = "ali")]
    Ali,
    #[strum(serialize = "andrew", serialize = "drew")]
    Andrew,
    #[strum(serialize = "keith")]
    Keith,
    #[strum(serialize = "art")]
    Art,
    #[strum(serialize = "nick")]
    Nick,
    Unknown,
}

impl std::fmt::Debug for ArchivedFriend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Austin => write!(f, "Austin"),
            Self::Jack => write!(f, "Jack"),
            Self::Sylvi => write!(f, "Sylvi"),
            Self::Ali => write!(f, "Ali"),
            Self::Andrew => write!(f, "Andrew"),
            Self::Keith => write!(f, "Keith"),
            Self::Art => write!(f, "Art"),
            Self::Nick => write!(f, "Nick"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(serde::Deserialize)]
pub struct Season {
    pub title: String,
    pub id: SeasonId,
    pub episodes: Vec<Episode>,
}

#[derive(serde::Deserialize)]
pub struct Episode {
    pub title: String,
    pub slug: String,
    pub done: bool,
    pub sorting_number: usize,
    pub docs_id: Option<String>,
    pub download: Option<DownloadOptions>,
}

#[derive(serde::Deserialize)]
pub struct DownloadOptions {
    pub plain: PathBuf,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, Clone)]
#[archive_attr(derive(Debug))]
pub struct StoredEpisode {
    pub id: u64,
    pub title: String,
    pub docs_id: Option<String>,
    pub slug: String,
    pub season: SeasonId,
    pub tokens: Vec<Sentence>,
    pub text: String,
    // pub terms_to_sentences: BTreeMap<u32, Vec<usize>>
}

#[derive(Debug, Error)]
pub enum CuriosityError {
    #[error(transparent)]
    QueryParserError(#[from] tantivy::query::QueryParserError),
    #[error(transparent)]
    Tantivy(#[from] tantivy::TantivyError),
    #[error(transparent)]
    TantivyOpenError(#[from] tantivy::directory::error::OpenDirectoryError),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    REDBError(#[from] redb::Error),
    #[error(transparent)]
    PostcardError(#[from] postcard::Error),
    #[error("not found")]
    NotFound,
}

pub type CuriosityResult<T> = Result<T, CuriosityError>;

mod term_map;
