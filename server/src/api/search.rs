use actix_web::{http::StatusCode, web, HttpResponse, HttpResponseBuilder};

use curiosity::db::Db;
use curiosity::docs_accessor::{DocsAccessor, DocumentGuard};
use curiosity::store::TermsToSentencesId;

use nyoom_json::{Serializer, UnescapedStr};
use redb::ReadableTable;

use crate::api::types::{QueryKind, SearchRequest};
use crate::{ServerError, ServerResult};

use tinyset::SetU32;

macro_rules! noescape {
    ($l:expr) => {
        // aesthetics
        UnescapedStr::create($l)
    };
}

#[actix_web::get("/search")]
pub async fn search(
    query: web::Query<SearchRequest>,
    db: web::Data<Db>,
) -> ServerResult<HttpResponse> {
    let query = query.into_inner();

    let mut query = if let Some(page) = query.page.as_ref().filter(|page| page.as_str() != "null") {
        let mut out = Vec::with_capacity(128);
        base64_url::decode_to_vec(&page, &mut out).map_err(|_| ServerError::BadPageToken)?;
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
    let sentences_db = txn.open_table(db.store.terms_to_sentences)?;
    let mut ep_db = db.store.get_docs_accessor(txn.get())?;

    let mut episodes = response_obj.array_field("episodes");
    for (doc_id, _) in results {
        let mut doc_reader = ep_db.get_doc(doc_id.0)?;
        let doc = doc_reader.read_doc();

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
            let Some(sentence_ids) = sentences_db.get(&term_to_sentence_id)? else {
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
