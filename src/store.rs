use std::{borrow::Cow, ops::Deref, sync::Arc};

use redb::{
    AccessGuard, MultimapTableDefinition, ReadOnlyTable, ReadableTable, RedbKey, RedbValue,
    TableDefinition, WriteTransaction,
};
use rend::u32_le;
use yoke::{Yoke, Yokeable};
use zerocopy::AsBytes;

use crate::{CuriosityError, CuriosityResult};

#[derive(Yokeable)]
#[repr(transparent)]
pub struct YokeableReadTransaction<'a> {
    inner: redb::ReadTransaction<'a>,
}

impl<'a> Deref for YokeableReadTransaction<'a> {
    type Target = redb::ReadTransaction<'a>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[repr(transparent)]
pub struct ReadTransaction {
    txn: Yoke<YokeableReadTransaction<'static>, Arc<redb::Database>>,
}

impl ReadTransaction {
    pub fn get(&self) -> &redb::ReadTransaction<'_> {
        self.txn.get().deref()
    }

    pub fn get_yokeable(&self) -> &YokeableReadTransaction<'_> {
        self.txn.get()
    }

    pub fn open_table<'a, K, V>(
        &'a self,
        v: TableDefinition<'_, K, V>,
    ) -> CuriosityResult<redb::ReadOnlyTable<'a, K, V>>
    where
        K: RedbKey + 'static,
        V: RedbValue + 'static,
    {
        self.txn
            .get()
            .open_table(v)
            .map_err(CuriosityError::REDBError)
    }

    pub fn open_multimap_table<'a, K, V>(
        &'a self,
        v: MultimapTableDefinition<'_, K, V>,
    ) -> CuriosityResult<redb::ReadOnlyMultimapTable<'a, K, V>>
    where
        K: RedbKey + 'static,
        V: RedbValue + RedbKey + 'static,
    {
        self.txn
            .get()
            .open_multimap_table(v)
            .map_err(CuriosityError::REDBError)
    }
}

pub struct DocsAccessor<'txn> {
    docs: ReadOnlyTable<'txn, u64, &'static [u8]>,
    terms_to_sentences: ReadOnlyTable<'txn, TermsToSentencesId, SentenceList<'static>>,
}

impl<'txn> DocsAccessor<'txn> {
    pub fn get_doc(&self, doc: u64) -> CuriosityResult<AccessGuard<&[u8]>> {
        self.docs.get(doc)?.ok_or(CuriosityError::NotFound)
    }

    pub fn get_sentences(
        &self,
        id: &TermsToSentencesId,
    ) -> CuriosityResult<Option<AccessGuard<SentenceList>>> {
        self.terms_to_sentences
            .get(id)
            .map_err(CuriosityError::REDBError)
    }
}

/* SPECIFICS */

#[derive(zerocopy::AsBytes, zerocopy::FromBytes, Debug)]
#[repr(C)]
pub struct TermsToSentencesId {
    doc: [u8; 8],
    term: [u8; 4],
}

impl TermsToSentencesId {
    pub fn new(doc: u64, term: u32) -> TermsToSentencesId {
        TermsToSentencesId {
            doc: doc.to_le_bytes(),
            term: term.to_le_bytes(),
        }
    }

    pub fn set_term(&mut self, term: u32) {
        self.term = term.to_le_bytes();
    }
}

impl RedbValue for TermsToSentencesId {
    type SelfType<'a>
     = &'a TermsToSentencesId where Self: 'a;

    type AsBytes<'a>
     = &'a [u8] where Self: 'a;

    fn fixed_width() -> Option<usize> {
        Some(std::mem::size_of::<TermsToSentencesId>())
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        debug_assert!(data.len() == std::mem::size_of::<TermsToSentencesId>());
        unsafe { &*(data.as_ptr() as *const TermsToSentencesId) }
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        value.as_bytes()
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("curiosityTermsToSentencesId")
    }
}

impl RedbKey for TermsToSentencesId {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        data1.cmp(data2)
    }
}

#[derive(Debug)]
pub struct SentenceList<'a> {
    pub ids: Cow<'a, [u32_le]>,
}

impl SentenceList<'_> {
    pub fn empty() -> Self {
        SentenceList {
            ids: Cow::Owned(Vec::new()),
        }
    }

    pub fn from_slice(v: &[u32]) -> Self {
        SentenceList {
            ids: Cow::Owned(v.iter().map(|v| u32_le::new(*v)).collect::<Vec<_>>()),
        }
    }
}

impl RedbValue for SentenceList<'_> {
    type SelfType<'a>
     = SentenceList<'a> where Self: 'a;

    type AsBytes<'a>
     = &'a [u8] where Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        let (_, data, _) = unsafe { data.align_to::<u32_le>() };
        SentenceList {
            ids: Cow::Borrowed(data),
        }
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        let (_, data, _) = unsafe { value.ids.align_to::<u8>() };
        data
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("curiositySentenceList")
    }
}

#[derive(Clone)]
pub struct Store {
    pub db: Arc<redb::Database>,
    pub docs: TableDefinition<'static, u64, &'static [u8]>,
    pub terms: TableDefinition<'static, &'static str, u32>,
    pub terms_to_sentences: TableDefinition<'static, TermsToSentencesId, SentenceList<'static>>,
}

impl Store {
    pub fn begin_read(&self) -> CuriosityResult<ReadTransaction> {
        Ok(ReadTransaction {
            txn: Yoke::try_attach_to_cart(Arc::clone(&self.db), |db| {
                db.begin_read()
                    .map(|v| YokeableReadTransaction { inner: v })
            })?,
        })
    }

    pub fn begin_write(&self) -> CuriosityResult<WriteTransaction> {
        self.db.begin_write().map_err(CuriosityError::REDBError)
    }

    pub fn get_docs_accessor<'a>(
        &self,
        txn: &'a redb::ReadTransaction<'a>,
    ) -> CuriosityResult<DocsAccessor<'a>> {
        Ok(DocsAccessor {
            docs: txn.open_table(self.docs)?,
            terms_to_sentences: txn.open_table(self.terms_to_sentences)?,
        })
    }
}
