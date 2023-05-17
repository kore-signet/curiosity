use serde::de::IntoDeserializer;

use smallvec::SmallVec;

use serde::Deserialize;

pub fn page_size_default() -> usize {
    50
}

// i love that serde makes me write these. i love it actually
pub fn is_false(b: &bool) -> bool {
    *b
}

// macro_rules! serde_try {
//     ($s:ty, $e:expr) => {
//         $e.map_err(|e| <$s>::Error::custom(e.to_string()))?
//     };
// }

// pub struct SerializableArchivedEpisode<'a, I: Iterator<Item = u32>> {
//     pub ep: &'a ArchivedStoredEpisode,
//     pub highlights: Option<HighlightsSerializer<'a, I>>,
// }

// impl<'a> SerializableArchivedEpisode<'a, std::iter::Empty<u32>> {
//     pub fn no_highlights(
//         ep: &'a ArchivedStoredEpisode,
//     ) -> SerializableArchivedEpisode<'a, std::iter::Empty<u32>> {
//         SerializableArchivedEpisode {
//             ep,
//             highlights: None,
//         }
//     }
// }

// impl<'a, I: Iterator<Item = u32>> SerializableArchivedEpisode<'a, I> {
//     pub fn new(
//         ep: &'a ArchivedStoredEpisode,
//         sentence_ids: I,
//         terms: &'a [u32],
//         is_phrase_query: bool,
//     ) -> SerializableArchivedEpisode<'a, I> {
//         SerializableArchivedEpisode {
//             ep,
//             highlights: Some(HighlightsSerializer {
//                 ep,
//                 sentence_ids: Cell::new(Some(sentence_ids)),
//                 terms,
//                 is_phrase_query,
//             }),
//         }
//     }
// }

// impl<'a, I: Iterator<Item = u32>> Serialize for SerializableArchivedEpisode<'a, I> {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         let mut state = serializer.serialize_struct("Episode", 6)?;
//         state.serialize_field("curiosity_id", &self.ep.id.value())?;
//         state.serialize_field("slug", &self.ep.slug.as_str())?;
//         state.serialize_field("title", self.ep.title.as_str())?;
//         if let ArchivedOption::Some(docs_id) = &self.ep.docs_id {
//             state.serialize_field("docs_id", &Some(docs_id.as_str()))?;
//         } else {
//             state.serialize_field("docs_id", &Option::<&str>::None)?;
//         }
//         state.serialize_field("season", &self.ep.season)?;
//         state.serialize_field("highlights", &self.highlights)?;
//         state.end()
//     }
// }

// pub struct HighlightsSerializer<'a, I: Iterator<Item = u32>> {
//     pub ep: &'a ArchivedStoredEpisode,
//     pub sentence_ids: Cell<Option<I>>,
//     pub terms: &'a [u32],
//     pub is_phrase_query: bool,
// }

// impl<'a, I: Iterator<Item = u32>> Serialize for HighlightsSerializer<'a, I> {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         let mut seq = serializer.serialize_seq(None)?;
//         for id in self
//             .sentence_ids
//             .take()
//             .ok_or_else(|| S::Error::custom("Highlights iterator already consumed!"))?
//         {
//             let sentence = &self.ep.tokens[id as usize];
//             if let Some(highlights) =
//                 sentence.highlight(self.terms, &self.ep.text, self.is_phrase_query)
//             {
//                 seq.serialize_element(&highlights)?;
//             }
//         }

//         seq.end()
//     }
// }

// pub struct ResultsSerializer<'a> {
//     pub(crate) inner: Cell<Option<ResultsSerializerState<'a>>>,
// }

// impl<'a> ResultsSerializer<'a> {
//     pub fn new(s: ResultsSerializerState<'a>) -> ResultsSerializer<'a> {
//         ResultsSerializer {
//             inner: Cell::new(Some(s)),
//         }
//     }
// }

// pub struct ResultsSerializerState<'a> {
//     pub query: QueryWithTerms<Box<dyn tantivy::query::Query>>,
//     pub highlight: bool,
//     pub is_phrase_query: bool,
//     pub results: Vec<(Reverse<u64>, DocAddress)>,
//     pub store: &'a Store,
// }

// impl<'a> Serialize for ResultsSerializer<'a> {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         let state = self
//             .inner
//             .take()
//             .ok_or_else(|| S::Error::custom("Results iterator already consumed!"))?;

//         let mut seq = serializer.serialize_seq(Some(state.results.len()))?;

//         let txn = serde_try!(S, state.store.begin_read());
//         let db = serde_try!(S, state.store.get_docs_accessor(txn.get()));

//         for (doc_id, _) in state.results {
//             let doc_bytes = serde_try!(S, db.get_doc(doc_id.0));
//             let doc = unsafe { rkyv::archived_root::<StoredEpisode>(doc_bytes.value()) };

//             if !state.highlight {
//                 seq.serialize_element(&SerializableArchivedEpisode::no_highlights(doc))?;
//                 continue;
//             }

//             let mut term_to_sentence_id = TermsToSentencesId::new(doc_id.0, 0);

//             let sentence_ids: BTreeSet<u32> =
//                 serde_try!(S,
//                 state
//                 .query
//                 .terms
//                 .iter()
//                 .filter_map(|term| {
//                     term_to_sentence_id.set_term(*term);
//                     db.get_sentences(&term_to_sentence_id).ok()
//                 })
//                 .flatten()
//                 .map(|res: Result<AccessGuard<u32>, redb::Error>| res.map(|acc| acc.value()))
//                 .collect::<Result<BTreeSet<u32>, redb::Error>>());
//             seq.serialize_element(&SerializableArchivedEpisode::new(
//                 doc,
//                 sentence_ids.into_iter(),
//                 &state.query.terms,
//                 state.is_phrase_query,
//             ))?;
//         }

//         seq.end()
//     }
// }

// https://github.com/actix/actix-web/issues/1301#issuecomment-747403932
pub fn deserialize_stringified_list<'de, D, I>(
    deserializer: D,
) -> std::result::Result<SmallVec<[I; 16]>, D::Error>
where
    D: serde::de::Deserializer<'de>,
    I: serde::de::DeserializeOwned,
{
    struct StringVecVisitor<I>(std::marker::PhantomData<I>);

    impl<'de, I> serde::de::Visitor<'de> for StringVecVisitor<I>
    where
        I: serde::de::DeserializeOwned,
    {
        type Value = SmallVec<[I; 16]>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string containing a list")
        }

        fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if v.is_empty() {
                return Ok(SmallVec::new());
            }

            let mut ids = SmallVec::new();
            for id in v.split(',') {
                let id = I::deserialize(id.into_deserializer())?;
                ids.push(id);
            }
            Ok(ids)
        }
    }

    if deserializer.is_human_readable() {
        deserializer.deserialize_any(StringVecVisitor(std::marker::PhantomData::<I>))
    } else {
        SmallVec::<[I; 16]>::deserialize(deserializer)
    }
}
