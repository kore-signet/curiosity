use curiosity::SeasonId;

use curiosity::serialization_crimes::*;

use serde::{Deserialize, Serialize};

use smallvec::SmallVec;
use smartstring::{Compact, SmartString};

#[derive(Serialize, Deserialize)]
pub struct SearchRequest {
    #[serde(alias = "q", default)]
    pub query: SmartString<Compact>,
    #[serde(default)]
    pub kind: QueryKind,
    #[serde(default, deserialize_with = "deserialize_stringified_list")]
    pub seasons: SmallVec<[SeasonId; 16]>,
    #[serde(default)]
    pub highlight: bool,
    #[serde(default)]
    pub _curiosity_internal_offset: usize,
    #[serde(default)]
    pub page: Option<String>,
    #[serde(default = "page_size_default")]
    pub page_size: usize,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum QueryKind {
    #[default]
    Keywords,
    Phrase,
    Web,
}
