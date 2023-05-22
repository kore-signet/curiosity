use redb::{AccessGuard, ReadOnlyTable, ReadableTable};

use crate::{ArchivedStoredEpisode, CuriosityError, CuriosityResult, StoredEpisode};

pub trait DocsAccessor {
    type Target<'a>: DocumentGuard
    where
        Self: 'a;
    fn get_doc(&mut self, doc: u64) -> CuriosityResult<Self::Target<'_>>;
}

pub trait DocumentGuard {
    type Target<'a>
    where
        Self: 'a;

    fn try_read_doc(&mut self) -> CuriosityResult<Self::Target<'_>>;

    /// Panicking version of try_read_doc
    fn read_doc(&mut self) -> Self::Target<'_>;
}

pub struct SimpleDocsAccessor<'txn> {
    pub(crate) docs: ReadOnlyTable<'txn, u64, &'static [u8]>,
}

impl<'txn> DocsAccessor for SimpleDocsAccessor<'txn> {
    type Target<'a> = AccessGuard<'a, &'a [u8]> where Self: 'a;

    #[inline(always)]
    fn get_doc(&mut self, doc: u64) -> CuriosityResult<AccessGuard<'_, &[u8]>> {
        self.docs.get(doc)?.ok_or(CuriosityError::NotFound)
    }
}

impl<'a> DocumentGuard for AccessGuard<'a, &'a [u8]> {
    type Target<'item>  = &'item ArchivedStoredEpisode where Self: 'item;

    #[inline(always)]
    fn try_read_doc(&mut self) -> CuriosityResult<Self::Target<'_>> {
        Ok(self.read_doc())
    }

    #[inline(always)]
    fn read_doc(&mut self) -> Self::Target<'_> {
        unsafe { rkyv::archived_root::<StoredEpisode>(self.value()) }
    }
}
