use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Arc, RwLock},
};

use boomphf::hashmap::BoomHashMap;

use redb::{MultimapTable, MultimapTableDefinition, ReadableTable, Table, TableDefinition};
use smallvec::SmallVec;
use smartstring::{LazyCompact, SmartString};
use tantivy::{
    collector::{FilterCollector, TopDocs},
    directory::MmapDirectory,
    query::{PhraseQuery, Query, QueryParser, TermSetQuery},
    store::Compressor,
    tokenizer::TextAnalyzer,
    DocAddress, DocId, Document, Index, IndexReader, IndexSettings, SegmentReader, Term,
};

use crate::{
    sentence::Sentence,
    store::{Store, TermsToSentencesId},
    CuriosityError, CuriosityResult, Episode, Season, SeasonId, StoredEpisode,
};

#[derive(Clone)]
pub struct Db {
    index: Index,
    reader: IndexReader,
    pub store: Store,
    parser: QueryParser,
    tokenizer: TextAnalyzer,
    term_map: Arc<RwLock<BoomHashMap<SmartString<LazyCompact>, u32>>>,
}

pub struct QueryWithTerms<T: Query> {
    pub query: T,
    pub terms: SmallVec<[u32; 8]>,
}

impl<T: Query> QueryWithTerms<T> {
    pub fn boxed(self) -> QueryWithTerms<Box<dyn Query>> {
        QueryWithTerms {
            query: Box::new(self.query),
            terms: self.terms,
        }
    }
}

impl Db {
    pub fn new(index_path: impl AsRef<Path>, store_path: impl AsRef<Path>) -> CuriosityResult<Db> {
        let index = Index::builder()
            .schema(crate::schema::build_schema())
            .settings(IndexSettings {
                docstore_compression: Compressor::None,
                ..Default::default()
            })
            .open_or_create(MmapDirectory::open(index_path)?)?;

        let store_env = redb::Database::builder()
            .set_cache_size(256_000_000)
            .create(store_path)?;

        let dbs = Store {
            db: Arc::new(store_env),
            docs: TableDefinition::new("docs"),
            terms: TableDefinition::new("terms"),
            terms_to_sentences: MultimapTableDefinition::new("terms_to_sentences"),
        };

        let txn = dbs.begin_read()?;

        let term_map = if let Ok(terms) = txn.open_table(dbs.terms) {
            let mut terms_keys: Vec<SmartString<LazyCompact>> = Vec::new();
            let mut terms_vals: Vec<u32> = Vec::new();

            for res in terms.iter()? {
                let (k, v) = res?;
                terms_keys.push(k.value().into());
                terms_vals.push(v.value());
            }

            Arc::new(RwLock::new(BoomHashMap::new(terms_keys, terms_vals)))
        } else {
            Arc::new(RwLock::new(BoomHashMap::new(Vec::new(), Vec::new())))
        };

        let reader = index.reader()?;
        let parser = QueryParser::for_index(
            &index,
            vec![
                index.schema().get_field("body").unwrap(),
                index.schema().get_field("title").unwrap(),
            ],
        );

        Ok(Db {
            tokenizer: index
                .tokenizer_for_field(index.schema().get_field("body").unwrap())
                .unwrap(),
            index,
            reader,
            store: dbs,
            parser,
            term_map,
        })
    }

    pub fn add_documents<'a, I, F>(&self, seasons: I, mut read_document: F) -> CuriosityResult<()>
    where
        I: IntoIterator<Item = &'a Season>,
        F: FnMut(SeasonId, &Episode) -> CuriosityResult<String>,
    {
        let txn = self.store.begin_write()?;
        txn.delete_table(self.store.docs)?;
        txn.delete_table(self.store.terms)?;
        txn.delete_multimap_table(self.store.terms_to_sentences)?;

        let mut terms_db: Table<&str, u32> = txn.open_table(self.store.terms)?;
        let mut doc_db: Table<u64, &[u8]> = txn.open_table(self.store.docs)?;
        let mut terms_to_sentences_db: MultimapTable<TermsToSentencesId, u32> =
            txn.open_multimap_table(self.store.terms_to_sentences)?;

        let mut index_writer = self.index.writer(100_000_000)?;
        index_writer.delete_all_documents()?;

        let schema = self.index.schema();
        let tokenizer = self
            .index
            .tokenizer_for_field(schema.get_field("body").unwrap())?;

        let mut term_map = HashMap::new();
        let mut authors = HashSet::new();

        for season in seasons {
            for episode in season.episodes.iter() {
                if episode.download.is_none() {
                    continue;
                }

                let ep_id = (season.id as u64 * 1000) + episode.sorting_number as u64;

                let episode_text = read_document(season.id, episode)?;

                let sentences = Sentence::tokenize(&episode_text, &tokenizer, &mut term_map)?;

                let stored_doc = StoredEpisode {
                    id: ep_id,
                    title: episode.title.clone(),
                    docs_id: episode.docs_id.clone(),
                    slug: episode.slug.clone(),
                    season: season.id,
                    tokens: sentences.clone(),
                    text: episode_text.clone(),
                };

                let serialized_doc = rkyv::util::to_bytes::<_, 1024>(&stored_doc).unwrap();
                doc_db.insert(ep_id, serialized_doc.as_slice())?;

                for (idx, sentence) in stored_doc.tokens.iter().enumerate() {
                    authors.insert(sentence.author);
                    for term in &sentence.tokens {
                        terms_to_sentences_db
                            .insert(&TermsToSentencesId::new(ep_id, term.term), idx as u32)?;
                    }
                }

                let mut doc = Document::new();
                doc.add_u64(schema.get_field("episode_id").unwrap(), ep_id);
                doc.add_text(schema.get_field("title").unwrap(), episode.title.as_str());
                doc.add_u64(schema.get_field("season").unwrap(), season.id as u64);
                doc.add_text(schema.get_field("body").unwrap(), episode_text.clone());

                index_writer.add_document(doc)?;
            }
        }

        let mut terms_keys: Vec<SmartString<LazyCompact>> = Vec::new();
        let mut terms_vals = Vec::new();

        for (k, v) in term_map {
            terms_db.insert(k.as_str(), v)?;

            terms_keys.push(k.into());
            terms_vals.push(v);
        }

        let mut term_map_guard = self.term_map.write().unwrap();

        *term_map_guard = BoomHashMap::new(terms_keys, terms_vals);

        drop(terms_db);
        drop(terms_to_sentences_db);
        drop(doc_db);

        txn.commit()?;
        index_writer.commit()?;

        Ok(())
    }

    pub fn parse_query(&self, query: &str) -> CuriosityResult<QueryWithTerms<impl Query>> {
        let query = self.parser.parse_query(query)?;
        let body_field = self.index.schema().get_field("body").unwrap();
        let mut terms = SmallVec::new();
        let term_map = self.term_map.read().unwrap();
        query.query_terms(&mut |term: &tantivy::Term, _| {
            if term.field() != body_field {
                return;
            }

            if let Some(term) = term.as_str().and_then(|text| term_map.get(text)) {
                terms.push(*term);
            }
        });

        terms.sort_unstable();

        Ok(QueryWithTerms { query, terms })
    }

    pub fn phrase_query(&self, query: &str) -> QueryWithTerms<impl Query> {
        let mut stream = self.tokenizer.token_stream(query);
        let mut out = Vec::with_capacity(query.len());
        let field = self.index.schema().get_field("body").unwrap();
        let term_map = self.term_map.read().unwrap();

        while let Some(tok) = stream.next() {
            out.push(Term::from_field_text(field, tok.text.as_str()));
        }

        let mut term_set = SmallVec::new();
        for term in out.iter() {
            if let Some(term) = term.as_str().and_then(|text| term_map.get(text)) {
                term_set.push(*term);
            }
        }

        QueryWithTerms {
            query: PhraseQuery::new(out),
            terms: term_set,
        }
    }

    pub fn keyword_query(&self, query: &str) -> QueryWithTerms<impl Query> {
        let mut stream = self.tokenizer.token_stream(query);
        let mut out = Vec::with_capacity(query.len());
        let field = self.index.schema().get_field("body").unwrap();
        let term_map = self.term_map.read().unwrap();

        while let Some(tok) = stream.next() {
            out.push(Term::from_field_text(field, tok.text.as_str()));
        }

        let mut term_set = SmallVec::new();
        for term in out.iter() {
            if let Some(term) = term.as_str().and_then(|text| term_map.get(text)) {
                term_set.push(*term);
            }
        }

        QueryWithTerms {
            query: TermSetQuery::new(out),
            terms: term_set,
        }
    }

    pub fn search(
        &self,
        query: &QueryWithTerms<impl Query>,
        filter_seasons: SmallVec<[SeasonId; 16]>,
        page_size: usize,
        offset: usize,
    ) -> CuriosityResult<Vec<(Reverse<u64>, DocAddress)>> {
        let episode_id_field = self.index.schema().get_field("episode_id").unwrap();
        let season_id_field = self.index.schema().get_field("season").unwrap();

        let searcher = self.reader.searcher();

        if !filter_seasons.is_empty() {
            searcher
                .search(
                    &query.query,
                    &FilterCollector::new(
                        season_id_field,
                        move |season: u64| {
                            let season_id: SeasonId = unsafe { std::mem::transmute(season) };

                            filter_seasons.contains(&season_id)
                        },
                        TopDocs::with_limit(page_size)
                            .and_offset(offset)
                            .custom_score(move |segment_reader: &SegmentReader| {
                                let episode_reader =
                                    segment_reader.fast_fields().u64(episode_id_field).unwrap();

                                move |doc: DocId| Reverse(episode_reader.get_val(doc))
                            }),
                    ),
                )
                .map_err(CuriosityError::Tantivy)
        } else {
            searcher
                .search(
                    &query.query,
                    &TopDocs::with_limit(page_size)
                        .and_offset(offset)
                        .custom_score(move |segment_reader: &SegmentReader| {
                            let episode_reader =
                                segment_reader.fast_fields().u64(episode_id_field).unwrap();

                            move |doc: DocId| Reverse(episode_reader.get_val(doc))
                        }),
                )
                .map_err(CuriosityError::Tantivy)
        }
    }
}

// pub struct ResultsIter<'a, I: Iterator<Item = (Reverse<u64>, DocAddress)>> {
//     pub store: &'a LmdbStores,
//     pub txn: LmdbTransaction,
//     pub searcher: Searcher,
//     episode_id_field: tantivy::schema::Field,
//     pub results: I,
//     pub results_len: usize,
// }

// impl<'b, I: Iterator<Item = (Reverse<u64>, DocAddress)>> LendingIterator for ResultsIter<'b, I> {
//     type Item<'a> = CuriosityResult<&'a ArchivedStoredEpisode> where Self: 'a;

//     fn next(&mut self) -> Option<Self::Item<'_>> {
//         let (_, addr) = self.results.next()?;
//         let doc = iter_try!(self.searcher.doc(addr));
//         let id = doc
//             .get_first(self.episode_id_field)
//             .and_then(|v| v.as_u64())
//             .unwrap();
//         let doc = iter_try!(self.txn.get().get(self.store.docs.db, &id.to_le_bytes()));
//         let doc_deser = unsafe { rkyv::util::archived_root::<StoredEpisode>(doc) };
//         Some(Ok(doc_deser))
//     }

//     fn len(&self) -> Option<usize> {
//         Some(self.results_len)
//     }
// }
