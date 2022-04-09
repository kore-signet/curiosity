use punkt::SentenceTokenizer;
use regex::{Regex, RegexBuilder, RegexSet};

lazy_static::lazy_static! {
    static ref TRAINING_DATA: punkt::TrainingData = punkt::TrainingData::english();
}

#[inline]
pub(crate) fn input_to_regex(text: &str) -> Regex {
    RegexBuilder::new(&regex::escape(text))
        .case_insensitive(true)
        .build()
        .unwrap()
}

pub(crate) fn highlights(
    text: &str,
    replacer: &str,
    keywords: &[Regex],
    keyword_set: &RegexSet,
    how_many: usize,
) -> Vec<String> {
    SentenceTokenizer::<punkt::params::Standard>::new(text, &TRAINING_DATA)
        .filter_map(|sentence| {
            if keyword_set.is_match(sentence) {
                let mut sentence = sentence.to_owned();

                for key in keywords {
                    sentence = key.replace_all(&sentence, replacer).to_string();
                }

                Some(sentence)
            } else {
                None
            }
        })
        .take(how_many)
        .collect()
}
