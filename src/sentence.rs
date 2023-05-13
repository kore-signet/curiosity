use std::{collections::HashMap, ops::Range, str::FromStr};

use line_span::LineSpans;

use rend::u32_le;
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

#[derive(Clone, Copy)]
pub struct CopyableTermRange {
    start: usize,
    end: usize,
    term: u32,
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
    pub author: Friend,
    pub start_in_original: usize,
    pub len: usize,
    pub tokens: Vec<SmallToken>,
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
                        term_map.insert(token.text.clone(), term_map.len() as u32);
                        term_map.len() as u32
                    };

                    // let term_id_bytes = term_id.to_le_bytes();
                    tokens.push(SmallToken {
                        start: token.offset_from,
                        end: token.offset_to,
                        term: term_id,
                        // term_text: token.text.clone()
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

                tokens.sort_by_key(|v| v.term);

                Ok(Sentence {
                    author,
                    start_in_original: sentence_start,
                    len: sentence.len(),
                    tokens,
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
        must_be_consecutive: bool,
    ) -> Option<SmallVec<[SentencePart<'b>; 8]>> {
        let mut ranges: SmallVec<[CopyableTermRange; 8]> = SmallVec::new();
        let mut found_count = 0;

        for term in terms {
            let mut token_idx = if let Ok(token_idx) = self
                .tokens
                .binary_search_by(|val| val.term.cmp(&u32_le::new(*term)))
            {
                found_count += 1;
                token_idx
            } else {
                continue;
            };

            loop {
                let token = &self.tokens[token_idx];
                if token.term != *term {
                    break;
                }

                ranges.push(CopyableTermRange {
                    start: token.start.value() as usize,
                    end: token.end.value() as usize,
                    term: *term,
                });

                if token_idx == 0 {
                    break;
                }

                token_idx -= 1;
            }
        }

        if found_count != terms.len() {
            return None;
        }

        ranges.sort_by_key(|v| v.start);
        let mut current_range = Range {
            start: 0usize,
            end: self.len.value() as usize,
        };
        let mut parts: SmallVec<[SentencePart<'_>; 8]> = SmallVec::new();

        let (start_in_original, sentence_len) = (
            self.start_in_original.value() as usize,
            self.len.value() as usize,
        );

        let ranges = collapse_overlapped_ranges(&ranges);

        if must_be_consecutive {
            if ranges.len() != terms.len() {
                return None;
            }

            for window in ranges.windows(terms.len()) {
                let mut end = window[0].end;
                for (idx, part) in window.iter().enumerate() {
                    if terms[idx] != part.term {
                        return None;
                    }

                    if part.start >= (end + 2) {
                        return None;
                    }

                    end = part.end;
                }
            }
        }

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

        Some(parts)
    }
}

#[inline(always)]
pub fn collapse_overlapped_ranges(
    ranges: &[CopyableTermRange],
) -> SmallVec<[CopyableTermRange; 8]> {
    let mut result = SmallVec::new();
    let mut ranges_it = ranges.iter();

    let mut current = match ranges_it.next() {
        Some(range) => *range,
        None => return result,
    };

    for range in ranges {
        if current.end > range.start && current.term == range.term {
            current = CopyableTermRange {
                start: current.start,
                end: std::cmp::max(current.end, range.end),
                term: current.term,
            };
        } else {
            result.push(current);
            current = *range;
        }
    }

    result.push(current);
    result
}
