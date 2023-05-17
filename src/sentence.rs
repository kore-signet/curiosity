use core::slice;
use std::{collections::HashMap, ops::Range, str::FromStr};

use line_span::LineSpans;

use memchr::memmem::Finder;
use nyoom_json::{ArrayWriter, JsonBuffer, UnescapedStr};
use rkyv::Archive;
use serde::ser::SerializeStruct;

use crate::{CuriosityResult, Friend};

use smallvec::SmallVec;
use tantivy::tokenizer::TextAnalyzer;

#[derive(Clone, Copy)]
pub struct CopyableRange {
    start: usize,
    end: usize,
}

impl CopyableRange {
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

impl From<CopyableRange> for Range<usize> {
    fn from(value: CopyableRange) -> Self {
        Range {
            start: value.start,
            end: value.end,
        }
    }
}

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, Clone)]
#[archive(archived = "ArchivedSentence")]
#[archive_attr(derive(Debug))]
pub struct Sentence {
    #[archive(skip)]
    pub author: Friend,
    pub start_in_original: usize,
    pub len: usize,
    pub tokens_by_position: Vec<SmallToken>,
    pub terms_by_position: Vec<u32>,
}

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, Clone)]
#[archive_attr(derive(Debug))]
pub struct SmallToken {
    pub start: usize,
    pub end: usize,
    pub term: u32,
    // pub term_text: String,
}

#[derive(Debug, Clone)]
pub enum SentencePart<'a> {
    Normal(&'a str),
    Highlighted(&'a str),
}

impl<'a> SentencePart<'a> {
    pub fn display_string(v: &[SentencePart<'a>]) -> String {
        let mut s = String::new();
        for part in v {
            match part {
                SentencePart::Normal(part) => s.push_str(part),
                SentencePart::Highlighted(part) => {
                    s.push('*');
                    s.push_str(part);
                    s.push('*');
                }
            }
        }

        s
    }

    pub fn is_highlighted(&self) -> bool {
        match self {
            SentencePart::Normal(_) => false,
            SentencePart::Highlighted(_) => true,
        }
    }

    pub fn text(&self) -> &str {
        match self {
            SentencePart::Normal(s) | SentencePart::Highlighted(s) => s,
        }
    }
}

impl<'a> serde::Serialize for SentencePart<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("SentencePart", 2)?;
        state.serialize_field("highlighted", &self.is_highlighted())?;
        state.serialize_field("text", self.text())?;
        state.end()
    }
}

impl Sentence {
    pub fn get<'a>(&self, body: &'a str) -> &'a str {
        &body[self.start_in_original..self.start_in_original + self.len]
    }

    pub fn tokenize(
        text: &str,
        tokenizer: &TextAnalyzer,
        term_map: &mut HashMap<String, u32>,
    ) -> CuriosityResult<Vec<Sentence>> {
        text.line_spans()
            .map(|line| -> CuriosityResult<Sentence> {
                let (sentence_start, sentence) = (line.start(), line.as_str());
                let mut tokens = Vec::new();
                let mut stream = tokenizer.token_stream(sentence);

                while let Some(token) = stream.next() {
                    let term_id = if let Some(term_id) = term_map.get(token.text.as_str()) {
                        *term_id
                    } else {
                        let term_id = term_map.len() as u32;
                        term_map.insert(token.text.clone(), term_map.len() as u32);
                        term_id
                    };

                    tokens.push(SmallToken {
                        start: token.offset_from,
                        end: token.offset_to,
                        term: term_id,
                    })
                }

                let author = Friend::from_str(
                    &sentence
                        .split_once(':')
                        .map_or("Unknown", |v| v.0)
                        .split_whitespace()
                        .next()
                        .unwrap_or("Unknown")
                        .to_lowercase(),
                )
                .unwrap_or(Friend::Unknown);

                tokens.sort_by_key(|v| v.start);

                let terms_by_position = tokens.iter().map(|v| v.term).collect::<Vec<_>>();

                Ok(Sentence {
                    author,
                    start_in_original: sentence_start,
                    len: sentence.len(),
                    tokens_by_position: tokens,
                    terms_by_position,
                })
            })
            .collect::<CuriosityResult<Vec<Sentence>>>()
    }
}

impl ArchivedSentence {
    pub fn highlight<'b>(
        &self,
        terms: &[u32],
        document: &'b str,
        is_phrase_query: bool,
    ) -> Option<HighlightedSentence<'b>> {
        let ranges = if is_phrase_query {
            let (found_count, ranges) = self.find_phrases(terms);
            if found_count == 0 {
                return None;
            }
            ranges
        } else {
            let (found_count, mut ranges) = self.find_keywords(terms);
            if found_count == 0 {
                return None;
            }

            ranges.sort_by_key(|v| v.start);

            collapse_overlapped_ranges(&ranges)
        };

        let mut current_range = Range {
            start: 0usize,
            end: self.len.value() as usize,
        };
        let mut parts: SmallVec<[SentencePart<'_>; 8]> = SmallVec::new();

        let (start_in_original, sentence_len) = (
            self.start_in_original.value() as usize,
            self.len.value() as usize,
        );

        for token_range in ranges {
            current_range.end = token_range.start;
            if !current_range.is_empty() {
                let start = start_in_original + current_range.start;
                let end = start_in_original + current_range.end;
                parts.push(SentencePart::Normal(&document[start..end]));
            }

            let start = start_in_original + token_range.start;
            let end = start_in_original + token_range.end;
            parts.push(SentencePart::Highlighted(&document[start..end]));

            current_range.start = token_range.end;
            current_range.end = sentence_len;
        }

        if !current_range.is_empty() {
            let start = start_in_original + current_range.start;
            let end = start_in_original + current_range.end;
            parts.push(SentencePart::Normal(&document[start..end]));
        }

        Some(HighlightedSentence(parts))
    }

    #[inline(always)]
    pub fn find_keywords(&self, terms: &[u32]) -> (usize, SmallVec<[CopyableRange; 8]>) {
        let mut ranges: SmallVec<[CopyableRange; 8]> = SmallVec::new();
        let mut found_count = 0;

        let haystack = unsafe {
            slice::from_raw_parts(
                self.terms_by_position.as_ptr() as *const u8,
                self.terms_by_position.len() * 4,
            )
        };

        for term in terms {
            for idx in memchr::memmem::find_iter(haystack, &term.to_le_bytes()) {
                found_count += 1;
                let idx = idx / 4;
                let token = &self.tokens_by_position[idx];
                ranges.push(CopyableRange {
                    start: token.start.value() as usize,
                    end: token.end.value() as usize,
                })
            }
        }

        (found_count, ranges)
    }

    #[inline(always)]
    pub fn find_phrases(&self, terms: &[u32]) -> (usize, SmallVec<[CopyableRange; 8]>) {
        let mut ranges: SmallVec<[CopyableRange; 8]> = SmallVec::new();
        let mut needle: SmallVec<[u8; 32]> = SmallVec::new();
        for term in terms {
            needle.extend_from_slice(&term.to_le_bytes());
        }
        let finder = Finder::new(needle.as_slice());

        let haystack = unsafe {
            slice::from_raw_parts(
                self.terms_by_position.as_ptr() as *const u8,
                self.terms_by_position.len() * 4,
            )
        };

        let mut found_count: usize = 0;
        for idx in finder.find_iter(haystack) {
            let idx = idx / 4;
            found_count += 1;
            let start_token = &self.tokens_by_position[idx];
            let end_token = &self.tokens_by_position[idx + terms.len() - 1];

            ranges.push(CopyableRange {
                start: start_token.start.value() as usize,
                end: end_token.end.value() as usize,
            })
        }

        (found_count, ranges)
    }
}

#[inline(always)]
pub fn collapse_overlapped_ranges(ranges: &[CopyableRange]) -> SmallVec<[CopyableRange; 8]> {
    let mut result = SmallVec::new();
    let mut ranges_it = ranges.iter();

    let mut current = match ranges_it.next() {
        Some(range) => *range,
        None => return result,
    };

    for range in ranges {
        if current.end > range.start {
            current = CopyableRange {
                start: current.start,
                end: std::cmp::max(current.end, range.end),
            };
        } else {
            result.push(current);
            current = *range;
        }
    }

    result.push(current);
    result
}

pub struct HighlightedSentence<'a>(SmallVec<[SentencePart<'a>; 8]>);

impl<'a> HighlightedSentence<'a> {
    pub fn serialize_into<S: JsonBuffer>(&self, mut ser: ArrayWriter<S>) {
        for part in &self.0 {
            let mut part_writer = ser.add_object();

            match part {
                SentencePart::Normal(text) => {
                    part_writer.field(UnescapedStr::create("text"), text);
                    part_writer.field(UnescapedStr::create("highlighted"), false);
                }
                SentencePart::Highlighted(text) => {
                    part_writer.field(UnescapedStr::create("text"), text);
                    part_writer.field(UnescapedStr::create("highlighted"), true);
                }
            }
        }
    }
}
