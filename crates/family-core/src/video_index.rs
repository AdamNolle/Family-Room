use crate::model::{MediaAsset, VideoIndex, VideoObservation};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use zeroize::Zeroize;

const NEARBY_MS: u64 = 4_000;

/// Export the device-local speech transcript as portable WebVTT captions.
/// Only recognized speech is included; OCR and classifier output are not
/// subtitles. The return count lets native clients explain empty transcripts.
pub(crate) fn speech_webvtt(index: &VideoIndex) -> (String, Vec<VideoObservation>) {
    let mut cues: Vec<_> = index
        .observations
        .iter()
        .filter(|item| item.modality == "speech" && !item.text.trim().is_empty())
        .map(|item| {
            let mut cue = item.clone();
            cue.text = cue.text.trim().to_owned();
            cue.end_ms = cue.end_ms.max(cue.start_ms.saturating_add(1));
            cue
        })
        .collect();
    cues.sort_by(|a, b| {
        a.start_ms
            .cmp(&b.start_ms)
            .then(a.end_ms.cmp(&b.end_ms))
            .then(a.text.cmp(&b.text))
    });
    cues.dedup_by(|a, b| a.start_ms == b.start_ms && a.end_ms == b.end_ms && a.text == b.text);

    let mut output = String::from("WEBVTT\n\n");
    for (number, cue) in cues.iter().enumerate() {
        let _ = writeln!(
            output,
            "{}\n{} --> {}\n{}\n",
            number + 1,
            vtt_time(cue.start_ms),
            vtt_time(cue.end_ms),
            escape_vtt_text(&cue.text)
        );
    }
    (output, cues)
}

fn vtt_time(milliseconds: u64) -> String {
    let hours = milliseconds / 3_600_000;
    let minutes = milliseconds / 60_000 % 60;
    let seconds = milliseconds / 1_000 % 60;
    let millis = milliseconds % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
}

fn escape_vtt_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace("-->", "--&gt;")
}

use std::fmt::Write as _;
const STOP_WORDS: &[&str] = &[
    "a",
    "an",
    "and",
    "about",
    "appears",
    "around",
    "at",
    "audio",
    "by",
    "can",
    "clip",
    "d",
    "did",
    "does",
    "exact",
    "find",
    "for",
    "frame",
    "frames",
    "from",
    "in",
    "is",
    "it",
    "look",
    "looking",
    "me",
    "moment",
    "movie",
    "of",
    "on",
    "please",
    "read",
    "reads",
    "re",
    "recording",
    "said",
    "say",
    "saying",
    "says",
    "scene",
    "screen",
    "show",
    "shows",
    "showing",
    "sign",
    "signs",
    "s",
    "someone",
    "spoken",
    "text",
    "that",
    "the",
    "there",
    "this",
    "t",
    "to",
    "video",
    "videos",
    "ve",
    "visible",
    "was",
    "what",
    "where",
    "while",
    "with",
    "word",
    "words",
    "written",
    "ll",
    "m",
    "you",
];

fn tokens(value: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut word = String::new();
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            word.push(character);
        } else if !word.is_empty() {
            result.push(std::mem::take(&mut word));
        }
    }
    if !word.is_empty() {
        result.push(word);
    }
    result
}

/// Normalize only lexical lookup terms. Exact-phrase checks keep using the
/// original transcript/OCR tokens so a paraphrase cannot become an exact hit.
fn search_tokens(value: &str) -> Vec<String> {
    tokens(value)
        .into_iter()
        .map(|token| searchable_token(&token))
        .collect()
}

fn searchable_token(token: &str) -> String {
    let mut stem = match token {
        "kid" | "kids" | "child" | "children" | "toddler" | "boy" | "boys" | "girl" | "girls" => {
            "child".to_owned()
        }
        "blew" | "blown" => "blow".to_owned(),
        "ran" => "run".to_owned(),
        "ate" | "eaten" => "eat".to_owned(),
        _ => token.to_owned(),
    };
    if stem.len() > 5 && stem.ends_with("ing") {
        stem.truncate(stem.len() - 3);
        collapse_doubled_ending(&mut stem);
        if stem.ends_with('c') || stem.ends_with('k') || stem.ends_with('v') {
            stem.push('e');
        }
    } else if stem.len() > 4 && stem.ends_with("ed") {
        stem.truncate(stem.len() - 2);
        collapse_doubled_ending(&mut stem);
        if stem.ends_with('k') || stem.ends_with('c') || stem.ends_with('v') {
            stem.push('e');
        }
    } else if stem.len() > 4 && stem.ends_with("ies") {
        stem.truncate(stem.len() - 3);
        stem.push('y');
    } else if stem.len() > 4
        && stem.ends_with("es")
        && stem.strip_suffix("es").is_some_and(|base| {
            ["s", "x", "z", "ch", "sh"]
                .iter()
                .any(|ending| base.ends_with(ending))
        })
    {
        stem.truncate(stem.len() - 2);
    } else if stem.len() > 3 && stem.ends_with('s') {
        stem.pop();
    }
    semantic_concept(&stem).to_owned()
}

/// Small, conservative concept families improve natural-language recall
/// without pretending to be a general-purpose embedding model. This
/// normalization only affects candidate retrieval; exact phrase checks use
/// the original words and never use these aliases.
fn semantic_concept(token: &str) -> &str {
    match token {
        "audience" | "crowd" | "group" | "people" => "group",
        "applaud" | "applause" | "clap" => "clap",
        "laugh" | "laughter" | "gigl" | "giggle" | "chuckl" | "chuckle" => "laugh",
        "sob" | "sobb" | "cri" | "cry" => "cry",
        "sing" | "song" => "sing",
        "dance" | "danc" => "dance",
        "jog" | "sprint" | "run" => "run",
        "hop" | "leap" | "jump" => "jump",
        "stroll" | "walk" => "walk",
        "hug" | "embrace" | "cuddl" | "cuddle" | "snuggl" | "snuggle" => "hug",
        "kiss" | "peck" | "smooch" => "kiss",
        "cheer" | "cheering" => "cheer",
        "shout" | "yell" | "scream" => "shout",
        "splash" | "splatter" => "splash",
        "bark" | "woof" => "bark",
        "meow" | "mew" => "meow",
        "present" | "gift" => "gift",
        "beach" | "coast" | "shore" | "seaside" => "beach",
        "snow" | "snowfall" => "snow",
        "firework" | "sparkler" | "sparkl" => "firework",
        _ => token,
    }
}

fn collapse_doubled_ending(stem: &mut String) {
    let mut characters = stem.chars().rev();
    if let (Some(last), Some(previous)) = (characters.next(), characters.next()) {
        if last == previous && last.is_ascii_alphabetic() {
            stem.pop();
        }
    }
}

/// Keep the quoted words as the exact phrase and preserve the surrounding
/// words as optional nearby context. Context can rank a result, but never
/// changes which words count as an exact transcript/OCR match.
fn quoted_phrase_parts(query: &str) -> Option<(&str, String)> {
    for (open, close) in [('"', '"'), ('“', '”'), ('‘', '’')] {
        let Some(open_at) = query.find(open) else {
            continue;
        };
        let start = open_at + open.len_utf8();
        let Some(close_offset) = query[start..].find(close) else {
            continue;
        };
        let close_at = start + close_offset;
        let phrase = query[start..close_at].trim();
        if !phrase.is_empty() {
            let context = format!(
                "{} {}",
                query[..open_at].trim(),
                query[close_at + close.len_utf8()..].trim()
            );
            return Some((phrase, context));
        }
    }
    let single_quotes: Vec<_> = query.match_indices('\'').map(|(index, _)| index).collect();
    for pair in single_quotes.windows(2) {
        let open_at = pair[0];
        let close_at = pair[1];
        let before = query[..open_at].chars().next_back();
        let after = query[close_at + 1..].chars().next();
        let opens_quote = before.is_none_or(|character| !character.is_alphanumeric());
        let closes_quote = after.is_none_or(|character| !character.is_alphanumeric());
        if opens_quote && closes_quote {
            let phrase = query[open_at + 1..close_at].trim();
            if !phrase.is_empty() {
                let context = format!(
                    "{} {}",
                    query[..open_at].trim(),
                    query[close_at + 1..].trim()
                );
                return Some((phrase, context));
            }
        }
    }
    None
}

struct SearchQuery {
    text: String,
    context_text: String,
    quoted: bool,
    browse_evidence: bool,
    browse_modality: Option<&'static str>,
    contextual: bool,
}

/// Keep evidence clauses when a request attributes a line to a speaker.
/// Attribution is not verified: the local index can locate words, but it
/// cannot identify who said them. Generic questions browse matching evidence.
fn search_query_base(query: &str) -> SearchQuery {
    if let Some((phrase, context_text)) = quoted_phrase_parts(query) {
        return SearchQuery {
            text: phrase.to_owned(),
            context_text,
            quoted: true,
            browse_evidence: false,
            browse_modality: None,
            contextual: false,
        };
    }

    let speech_verbs = [
        "say", "says", "said", "saying", "ask", "asks", "asked", "asking", "sing", "sings", "sang",
        "singing", "shout", "shouts", "shouted", "shouting", "yell", "yells", "yelled", "yelling",
        "tell", "tells", "told", "telling",
    ];
    let mut spans = Vec::<(usize, usize, String)>::new();
    let mut word_start = None;
    for (offset, character) in query.char_indices() {
        if character.is_alphanumeric() {
            word_start.get_or_insert(offset);
        } else if let Some(start) = word_start.take() {
            spans.push((start, offset, query[start..offset].to_lowercase()));
        }
    }
    if let Some(start) = word_start {
        spans.push((start, query.len(), query[start..].to_lowercase()));
    }

    let clause = spans.iter().find_map(|(_, end, word)| {
        speech_verbs
            .contains(&word.as_str())
            .then(|| query[*end..].trim_matches(|character: char| !character.is_alphanumeric()))
            .filter(|suffix| !suffix.is_empty())
    });
    let query_tokens = tokens(query);
    let starts_as_question = query_tokens
        .first()
        .is_some_and(|word| matches!(word.as_str(), "what" | "where" | "which" | "show"));
    let has_frame_cue = query_tokens.iter().any(|word| {
        matches!(
            word.as_str(),
            "frame"
                | "frames"
                | "read"
                | "reads"
                | "screen"
                | "sign"
                | "signs"
                | "text"
                | "word"
                | "words"
                | "written"
        )
    });
    let asks_for_unfiltered_speech = starts_as_question
        && !has_frame_cue
        && spans.iter().any(|(_, end, word)| {
            speech_verbs.contains(&word.as_str())
                && query[*end..]
                    .trim_matches(|character: char| !character.is_alphanumeric())
                    .is_empty()
        });
    let asks_for_unfiltered_frame_text = starts_as_question
        && has_frame_cue
        && query_tokens
            .iter()
            .all(|word| STOP_WORDS.contains(&word.as_str()));
    let browse_evidence = asks_for_unfiltered_speech || asks_for_unfiltered_frame_text;
    let browse_modality = if asks_for_unfiltered_frame_text {
        Some("frame")
    } else if asks_for_unfiltered_speech {
        Some("audio")
    } else {
        None
    };

    SearchQuery {
        text: if browse_evidence {
            String::new()
        } else {
            clause.unwrap_or(query).trim().to_owned()
        },
        context_text: String::new(),
        quoted: false,
        browse_evidence,
        browse_modality,
        contextual: false,
    }
}

fn is_contextual_followup(query: &str) -> bool {
    let normalized = query.trim().to_lowercase();
    ["what about ", "how about ", "and ", "also "]
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
        || ["in that video", "in that clip"]
            .iter()
            .any(|phrase| normalized.contains(phrase))
}

fn search_query(query: &str, context_query: Option<&str>) -> SearchQuery {
    let mut current = search_query_base(query);
    let Some(context_query) = context_query
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return current;
    };
    // A current quoted request is always an exact, standalone lookup.
    if current.quoted || current.browse_evidence || !is_contextual_followup(query) {
        return current;
    }
    let context = search_query_base(context_query);
    if context.text.trim().is_empty() {
        return current;
    }
    current.context_text = format!(
        "{} {} {}",
        current.context_text, context.text, context.context_text
    )
    .trim()
    .to_owned();
    current.contextual = true;
    current
}

fn normalized_query_terms(query: &SearchQuery) -> Vec<String> {
    let raw_terms: Vec<_> = tokens(&query.text)
        .into_iter()
        .chain(tokens(&query.context_text))
        .collect();
    let mut terms: Vec<_> = raw_terms
        .iter()
        .filter(|term| !STOP_WORDS.contains(&term.as_str()))
        .map(|term| searchable_token(term))
        .collect();
    if terms.is_empty() && !query.browse_evidence {
        terms = raw_terms
            .iter()
            .map(|term| searchable_token(term))
            .collect();
    }
    terms.sort();
    terms.dedup();
    terms
}

fn normalized_context_terms(query: &SearchQuery) -> Vec<String> {
    let phrase_terms: HashSet<_> = tokens(&query.text)
        .into_iter()
        .map(|term| searchable_token(&term))
        .collect();
    let mut terms: Vec<_> = tokens(&query.context_text)
        .into_iter()
        .filter(|term| !STOP_WORDS.contains(&term.as_str()))
        .map(|term| searchable_token(&term))
        .filter(|term| !phrase_terms.contains(term))
        .collect();
    terms.sort();
    terms.dedup();
    terms
}

#[derive(Clone, Copy)]
struct ExactPhraseMatch<'a> {
    start_ms: u64,
    end_ms: u64,
    verified_text: bool,
    modality: &'a str,
}

/// Detect an ordered phrase across adjacent observations in the same medium.
/// This supports a spoken sentence split at an indexing boundary and OCR that
/// reports a sign as several lines, without joining unrelated moments. A
/// transcript or OCR stream can verify exact wording; classifier labels cannot.
fn exact_phrase_match<'a>(
    observations: impl Iterator<Item = &'a VideoObservation>,
    phrase: &str,
    ignore_stop_words: bool,
) -> Option<ExactPhraseMatch<'a>> {
    let target: Vec<_> = tokens(phrase)
        .into_iter()
        .filter(|word| !ignore_stop_words || !STOP_WORDS.contains(&word.as_str()))
        .collect();
    if target.is_empty() {
        return None;
    }
    let mut streams = HashMap::<&'a str, (u64, VecDeque<(String, u64, u64)>)>::new();
    let mut best_match = None;
    for observation in observations {
        let (last_end, words) = streams
            .entry(observation.modality.as_str())
            .or_insert_with(|| (0, VecDeque::new()));
        if observation.start_ms > (*last_end).saturating_add(1_000) {
            words.clear();
        }
        *last_end = (*last_end).max(observation.end_ms);
        for word in tokens(&observation.text)
            .into_iter()
            .filter(|word| !ignore_stop_words || !STOP_WORDS.contains(&word.as_str()))
        {
            words.push_back((word, observation.start_ms, observation.end_ms));
            if words.len() > target.len() {
                words.pop_front();
            }
            if words.len() == target.len()
                && words
                    .iter()
                    .map(|(word, _, _)| word.as_str())
                    .eq(target.iter().map(String::as_str))
            {
                let start_ms = words.iter().map(|(_, start, _)| *start).min().unwrap_or(0);
                let end_ms = words
                    .iter()
                    .map(|(_, _, end)| *end)
                    .max()
                    .unwrap_or(start_ms);
                let candidate = ExactPhraseMatch {
                    start_ms,
                    end_ms,
                    verified_text: matches!(
                        observation.modality.as_str(),
                        "speech" | "screen_text"
                    ),
                    modality: &observation.modality,
                };
                if best_match.is_none_or(|current: ExactPhraseMatch<'_>| {
                    candidate.verified_text && !current.verified_text
                }) {
                    best_match = Some(candidate);
                }
            }
        }
    }
    best_match
}

fn selectable(observation: &VideoObservation, filter: &str) -> bool {
    match filter {
        "audio" => observation.modality == "speech" || observation.modality == "sound_event",
        "frame" => observation.modality == "screen_text" || observation.modality == "visual_label",
        _ => true,
    }
}

fn minimum_suggestion_terms(query_terms: usize) -> usize {
    if query_terms <= 1 {
        1
    } else {
        query_terms.div_ceil(2).max(2)
    }
}

struct SearchEntry<'a> {
    observation: &'a VideoObservation,
    term_hits: Vec<usize>,
}

const MAX_CACHED_OBSERVATIONS: usize = 100_000;
const MAX_CACHED_TEXT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy)]
struct Posting {
    video: usize,
    observation: usize,
}

struct CachedVideo {
    sorted: Vec<usize>,
    observation_terms: Vec<Vec<usize>>,
    max_span: u64,
}

/// A bounded, in-memory inverted index. The encrypted catalog remains the
/// source of truth; token postings are never written to a plaintext file.
pub struct VideoSearchCache {
    lexicon: HashMap<String, usize>,
    postings: Vec<Vec<Posting>>,
    videos: Vec<CachedVideo>,
}

impl Drop for VideoSearchCache {
    fn drop(&mut self) {
        for (mut token, _) in self.lexicon.drain() {
            token.zeroize();
        }
    }
}

impl VideoSearchCache {
    pub fn build(indexes: &[&VideoIndex]) -> Option<Self> {
        Self::build_limited(indexes, MAX_CACHED_OBSERVATIONS, MAX_CACHED_TEXT_BYTES)
    }

    fn build_limited(
        indexes: &[&VideoIndex],
        max_observations: usize,
        max_text_bytes: usize,
    ) -> Option<Self> {
        let mut count = 0usize;
        let mut text_bytes = 0usize;
        for index in indexes {
            count = count.saturating_add(index.observations.len());
            text_bytes = text_bytes.saturating_add(
                index
                    .observations
                    .iter()
                    .map(|entry| entry.text.len())
                    .sum::<usize>(),
            );
            if count > max_observations || text_bytes > max_text_bytes {
                return None;
            }
        }
        let mut cache = Self {
            lexicon: HashMap::new(),
            postings: Vec::new(),
            videos: Vec::with_capacity(indexes.len()),
        };
        for (video, index) in indexes.iter().enumerate() {
            let mut sorted: Vec<_> = (0..index.observations.len()).collect();
            sorted.sort_by_key(|&position| index.observations[position].start_ms);
            let mut observation_terms = Vec::with_capacity(index.observations.len());
            for (observation, entry) in index.observations.iter().enumerate() {
                let mut term_ids = Vec::new();
                for token in search_tokens(&entry.text) {
                    let term_id = if let Some(&existing) = cache.lexicon.get(&token) {
                        existing
                    } else {
                        let next = cache.postings.len();
                        cache.lexicon.insert(token, next);
                        cache.postings.push(Vec::new());
                        next
                    };
                    term_ids.push(term_id);
                }
                term_ids.sort_unstable();
                term_ids.dedup();
                for &term_id in &term_ids {
                    cache.postings[term_id].push(Posting { video, observation });
                }
                observation_terms.push(term_ids);
            }
            let max_span = index
                .observations
                .iter()
                .map(|entry| entry.end_ms.saturating_sub(entry.start_ms))
                .max()
                .unwrap_or(0);
            cache.videos.push(CachedVideo {
                sorted,
                observation_terms,
                max_span,
            });
        }
        Some(cache)
    }

    pub fn search(
        &self,
        indexes: &[&VideoIndex],
        assets: &[MediaAsset],
        query: &str,
        filter: &str,
    ) -> Value {
        self.search_with_context(indexes, assets, query, filter, None)
    }

    pub fn search_with_context(
        &self,
        indexes: &[&VideoIndex],
        assets: &[MediaAsset],
        query: &str,
        filter: &str,
        context_query: Option<&str>,
    ) -> Value {
        let query = search_query(query, context_query);
        let intent_mismatch = query
            .browse_modality
            .is_some_and(|intent| filter != "all" && filter != intent);
        let filter = if filter == "all" {
            query.browse_modality.unwrap_or(filter)
        } else {
            filter
        };
        let visible: HashSet<_> = assets
            .iter()
            .filter(|asset| asset.kind == "video" && !asset.deleted)
            .map(|asset| (asset.id.as_str(), asset.sha256.as_str()))
            .collect();
        let allowed: Vec<_> = indexes
            .iter()
            .map(|index| visible.contains(&(index.asset_id.as_str(), index.asset_sha256.as_str())))
            .collect();
        let indexed_count = allowed.iter().filter(|&&permitted| permitted).count();
        let audio_count = indexes
            .iter()
            .zip(&allowed)
            .filter(|(index, allowed)| **allowed && index.audio_indexed)
            .count();
        let frame_count = indexes
            .iter()
            .zip(&allowed)
            .filter(|(index, allowed)| **allowed && index.frames_indexed)
            .count();
        let sound_event_count = indexes
            .iter()
            .zip(&allowed)
            .filter(|(index, allowed)| **allowed && index.sound_events_indexed)
            .count();
        if intent_mismatch {
            return contextual_result(
                finish(
                    Vec::new(),
                    visible.len(),
                    indexed_count,
                    audio_count,
                    frame_count,
                    sound_event_count,
                ),
                query.contextual,
            );
        }
        let terms = normalized_query_terms(&query);
        let context_terms = normalized_context_terms(&query);
        let term_ids: Vec<_> = terms
            .iter()
            .filter_map(|term| self.lexicon.get(term).copied())
            .collect();
        let context_term_ids: Vec<_> = context_terms
            .iter()
            .filter_map(|term| self.lexicon.get(term).copied())
            .collect();
        if term_ids.is_empty() && !query.browse_evidence {
            return contextual_result(
                finish(
                    Vec::new(),
                    visible.len(),
                    indexed_count,
                    audio_count,
                    frame_count,
                    sound_event_count,
                ),
                query.contextual,
            );
        };
        let mut anchors = vec![Vec::<usize>::new(); indexes.len()];
        if query.browse_evidence {
            for (video, index) in indexes.iter().enumerate() {
                if allowed[video] {
                    for (observation, entry) in index.observations.iter().enumerate() {
                        if selectable(entry, filter) {
                            anchors[video].push(observation);
                        }
                    }
                }
            }
        } else {
            for &term_id in &term_ids {
                for posting in &self.postings[term_id] {
                    if allowed[posting.video]
                        && selectable(
                            &indexes[posting.video].observations[posting.observation],
                            filter,
                        )
                    {
                        anchors[posting.video].push(posting.observation);
                    }
                }
            }
        }
        let mut matches = Vec::new();
        let mut suggestions = Vec::new();
        for (video, candidates) in anchors.iter_mut().enumerate() {
            if candidates.is_empty() {
                continue;
            }
            let index = indexes[video];
            let cached = &self.videos[video];
            candidates.sort_unstable_by_key(|&position| {
                (index.observations[position].start_ms, position)
            });
            candidates.dedup();
            for &position in candidates.iter() {
                let anchor = &index.observations[position];
                let start = anchor
                    .start_ms
                    .saturating_sub(cached.max_span.saturating_add(NEARBY_MS));
                let end = anchor.end_ms.saturating_add(NEARBY_MS);
                let first = cached
                    .sorted
                    .partition_point(|&entry| index.observations[entry].start_ms < start);
                let last = cached
                    .sorted
                    .partition_point(|&entry| index.observations[entry].start_ms <= end);
                let nearby: Vec<_> = cached.sorted[first..last]
                    .iter()
                    .copied()
                    .filter(|&entry| {
                        let observation = &index.observations[entry];
                        selectable(observation, filter)
                            && observation.end_ms.saturating_add(NEARBY_MS) >= anchor.start_ms
                    })
                    .collect();
                let phrase_match = (!query.browse_evidence && !query.contextual)
                    .then(|| {
                        exact_phrase_match(
                            nearby.iter().map(|&entry| &index.observations[entry]),
                            &query.text,
                            !query.quoted,
                        )
                    })
                    .flatten();
                let exact_phrase = phrase_match.is_some();
                if !query.browse_evidence && query.quoted && !exact_phrase {
                    continue;
                }
                let matched_terms = term_ids
                    .iter()
                    .filter(|term_id| {
                        nearby.iter().any(|&entry| {
                            cached.observation_terms[entry]
                                .binary_search(term_id)
                                .is_ok()
                        })
                    })
                    .count();
                let is_exact =
                    phrase_match.is_some_and(|matched| matched.verified_text) && !query.contextual;
                let is_evidence = query.browse_evidence
                    && matches!(anchor.modality.as_str(), "speech" | "screen_text");
                if !is_exact
                    && !is_evidence
                    && !query.browse_evidence
                    && !query.quoted
                    && matched_terms < minimum_suggestion_terms(terms.len())
                {
                    continue;
                }
                let relevant: Vec<_> = nearby
                    .iter()
                    .copied()
                    .filter(|&entry| {
                        query.browse_evidence
                            || term_ids.iter().any(|term_id| {
                                cached.observation_terms[entry]
                                    .binary_search(term_id)
                                    .is_ok()
                            })
                    })
                    .collect::<Vec<_>>();
                let evidence: Vec<_> = relevant
                    .iter()
                    .take(8)
                    .map(|&entry| {
                        serde_json::to_value(&index.observations[entry]).unwrap_or(Value::Null)
                    })
                    .collect();
                let context_evidence: Vec<_> = nearby
                    .iter()
                    .copied()
                    .filter(|&entry| {
                        context_term_ids.iter().any(|term_id| {
                            cached.observation_terms[entry]
                                .binary_search(term_id)
                                .is_ok()
                        })
                    })
                    .take(4)
                    .map(|entry| {
                        serde_json::to_value(&index.observations[entry]).unwrap_or(Value::Null)
                    })
                    .collect();
                let start_ms = phrase_match
                    .map(|matched| matched.start_ms)
                    .unwrap_or_else(|| {
                        relevant
                            .iter()
                            .map(|&entry| index.observations[entry].start_ms)
                            .min()
                            .unwrap_or(anchor.start_ms)
                    });
                let end_ms = phrase_match
                    .map(|matched| matched.end_ms)
                    .unwrap_or_else(|| {
                        relevant
                            .iter()
                            .map(|&entry| index.observations[entry].end_ms)
                            .max()
                            .unwrap_or(anchor.end_ms)
                    });
                let score = 10 * matched_terms as u64
                    + u64::from(exact_phrase) * 100
                    + u64::from(anchor.confidence_percent);
                let result = json!({
                    "room_id":index.room_id,"asset_id":index.asset_id,
                    "start_ms":start_ms,"end_ms":end_ms,"score":score,"evidence":evidence,
                    "context_evidence":context_evidence,
                    "phrase_modality":phrase_match.map(|matched| matched.modality),
                    "match_kind":if is_exact { "exact" } else if is_evidence { "evidence" } else { "suggestion" },
                "matched_terms":matched_terms,"query_terms":terms.len()
                });
                if is_exact || is_evidence {
                    matches.push(result);
                } else {
                    suggestions.push(result);
                }
            }
        }
        if matches.is_empty() {
            matches = suggestions;
        }
        contextual_result(
            finish(
                matches,
                visible.len(),
                indexed_count,
                audio_count,
                frame_count,
                sound_event_count,
            ),
            query.contextual,
        )
    }
}

pub fn search(indexes: &[&VideoIndex], assets: &[MediaAsset], query: &str, filter: &str) -> Value {
    search_with_context(indexes, assets, query, filter, None)
}

pub fn search_with_context(
    indexes: &[&VideoIndex],
    assets: &[MediaAsset],
    query: &str,
    filter: &str,
    context_query: Option<&str>,
) -> Value {
    let query = search_query(query, context_query);
    let intent_mismatch = query
        .browse_modality
        .is_some_and(|intent| filter != "all" && filter != intent);
    let filter = if filter == "all" {
        query.browse_modality.unwrap_or(filter)
    } else {
        filter
    };
    let visible: HashSet<_> = assets
        .iter()
        .filter(|asset| asset.kind == "video" && !asset.deleted)
        .map(|asset| (asset.id.as_str(), asset.sha256.as_str()))
        .collect();
    let indexed: Vec<_> = indexes
        .iter()
        .filter(|index| visible.contains(&(index.asset_id.as_str(), index.asset_sha256.as_str())))
        .collect();
    let sound_event_count = indexed
        .iter()
        .filter(|entry| entry.sound_events_indexed)
        .count();
    if intent_mismatch {
        return contextual_result(
            finish(
                Vec::new(),
                visible.len(),
                indexed.len(),
                indexed.iter().filter(|entry| entry.audio_indexed).count(),
                indexed.iter().filter(|entry| entry.frames_indexed).count(),
                sound_event_count,
            ),
            query.contextual,
        );
    }
    let terms = normalized_query_terms(&query);
    let context_terms = normalized_context_terms(&query);
    let context_positions: Vec<_> = context_terms
        .iter()
        .filter_map(|term| terms.binary_search(term).ok())
        .collect();
    let mut matches = Vec::<Value>::new();
    let mut suggestions = Vec::<Value>::new();
    for index in &indexed {
        // Tokenize each observation once and scan only bounded time windows
        // around observations containing a query term.
        let mut entries: Vec<_> = index
            .observations
            .iter()
            .filter(|entry| selectable(entry, filter))
            .map(|observation| {
                let mut term_hits: Vec<_> = search_tokens(&observation.text)
                    .into_iter()
                    .filter_map(|token| terms.binary_search(&token).ok())
                    .collect();
                term_hits.sort_unstable();
                term_hits.dedup();
                SearchEntry {
                    observation,
                    term_hits,
                }
            })
            .collect();
        entries.sort_by_key(|entry| entry.observation.start_ms);
        let max_span = entries
            .iter()
            .map(|entry| {
                entry
                    .observation
                    .end_ms
                    .saturating_sub(entry.observation.start_ms)
            })
            .max()
            .unwrap_or(0);
        for anchor in entries
            .iter()
            .filter(|entry| query.browse_evidence || !entry.term_hits.is_empty())
        {
            let start = anchor
                .observation
                .start_ms
                .saturating_sub(max_span.saturating_add(NEARBY_MS));
            let end = anchor.observation.end_ms.saturating_add(NEARBY_MS);
            let first = entries.partition_point(|entry| entry.observation.start_ms < start);
            let last = entries.partition_point(|entry| entry.observation.start_ms <= end);
            let nearby: Vec<_> = entries[first..last]
                .iter()
                .filter(|entry| {
                    entry.observation.end_ms.saturating_add(NEARBY_MS)
                        >= anchor.observation.start_ms
                })
                .collect();
            let phrase_match = (!query.browse_evidence && !query.contextual)
                .then(|| {
                    exact_phrase_match(
                        nearby.iter().map(|entry| entry.observation),
                        &query.text,
                        !query.quoted,
                    )
                })
                .flatten();
            let exact_phrase = phrase_match.is_some();
            if query.quoted && !exact_phrase {
                continue;
            }
            let mut found = vec![false; terms.len()];
            for entry in &nearby {
                for &hit in &entry.term_hits {
                    found[hit] = true;
                }
            }
            let matched_terms = found.iter().filter(|matched| **matched).count();
            let is_exact =
                phrase_match.is_some_and(|matched| matched.verified_text) && !query.contextual;
            let is_evidence = query.browse_evidence
                && matches!(
                    anchor.observation.modality.as_str(),
                    "speech" | "screen_text"
                );
            if !is_exact
                && !is_evidence
                && !query.browse_evidence
                && !query.quoted
                && matched_terms < minimum_suggestion_terms(terms.len())
            {
                continue;
            }
            let relevant: Vec<_> = nearby
                .iter()
                .filter(|entry| query.browse_evidence || !entry.term_hits.is_empty())
                .collect();
            let evidence: Vec<_> = relevant
                .iter()
                .take(8)
                .map(|entry| serde_json::to_value(entry.observation).unwrap_or(Value::Null))
                .collect();
            let context_evidence: Vec<_> = nearby
                .iter()
                .filter(|entry| {
                    context_positions
                        .iter()
                        .any(|position| entry.term_hits.binary_search(position).is_ok())
                })
                .take(4)
                .map(|entry| serde_json::to_value(entry.observation).unwrap_or(Value::Null))
                .collect();
            let start_ms = phrase_match
                .map(|matched| matched.start_ms)
                .unwrap_or_else(|| {
                    relevant
                        .iter()
                        .map(|entry| entry.observation.start_ms)
                        .min()
                        .unwrap_or(anchor.observation.start_ms)
                });
            let end_ms = phrase_match
                .map(|matched| matched.end_ms)
                .unwrap_or_else(|| {
                    relevant
                        .iter()
                        .map(|entry| entry.observation.end_ms)
                        .max()
                        .unwrap_or(anchor.observation.end_ms)
                });
            let score = 10 * matched_terms as u64
                + u64::from(exact_phrase) * 100
                + u64::from(anchor.observation.confidence_percent);
            let result = json!({
                "room_id":index.room_id,"asset_id":index.asset_id,
                "start_ms":start_ms,"end_ms":end_ms,"score":score,"evidence":evidence,
                "context_evidence":context_evidence,
                "phrase_modality":phrase_match.map(|matched| matched.modality),
                "match_kind":if is_exact { "exact" } else if is_evidence { "evidence" } else { "suggestion" },
                "matched_terms":matched_terms,"query_terms":terms.len()
            });
            if is_exact || is_evidence {
                matches.push(result);
            } else {
                suggestions.push(result);
            }
        }
    }
    if matches.is_empty() {
        matches = suggestions;
    }
    contextual_result(
        finish(
            matches,
            visible.len(),
            indexed.len(),
            indexed.iter().filter(|entry| entry.audio_indexed).count(),
            indexed.iter().filter(|entry| entry.frames_indexed).count(),
            sound_event_count,
        ),
        query.contextual,
    )
}

fn contextual_result(mut result: Value, contextual: bool) -> Value {
    if contextual {
        result["contextual_search"] = json!(true);
        if let Some(answer) = result["answer"].as_str() {
            result["answer"] = json!(format!(
                "Using your previous video question as context. {answer}"
            ));
        }
    }
    result
}

fn evidence_answer(result: &Value) -> Option<String> {
    let evidence = result["evidence"].as_array()?;
    let observation = result["phrase_modality"]
        .as_str()
        .and_then(|modality| evidence.iter().find(|entry| entry["modality"] == modality))
        .or_else(|| evidence.first())?;
    let text = observation["text"].as_str()?.trim();
    if text.is_empty() {
        return None;
    }

    let modality = match observation["modality"].as_str().unwrap_or_default() {
        "speech" => "spoken words",
        "screen_text" => "text on screen",
        "visual_label" => "a visual match",
        "sound_event" => "a possible sound",
        _ => "indexed evidence",
    };
    let start_ms = observation["start_ms"]
        .as_u64()
        .or_else(|| result["start_ms"].as_u64())
        .unwrap_or_default();
    let seconds = start_ms / 1_000;
    let tenth = (start_ms % 1_000) / 100;
    let timestamp = if seconds >= 3_600 {
        format!(
            "{:02}:{:02}:{:02}.{}",
            seconds / 3_600,
            (seconds / 60) % 60,
            seconds % 60,
            tenth
        )
    } else {
        format!("{:02}:{:02}.{}", seconds / 60, seconds % 60, tenth)
    };
    let mut excerpt: String = text.chars().take(180).collect();
    if text.chars().count() > 180 {
        excerpt.push('…');
    }
    let mut answer = if result["match_kind"] == "suggestion" {
        let matched = result["matched_terms"].as_u64().unwrap_or_default();
        let total = result["query_terms"].as_u64().unwrap_or_default();
        format!(
            "No exact match. Closest indexed evidence matched {matched} of {total} search terms in {modality} at {timestamp}: “{excerpt}”. Open the video to verify."
        )
    } else {
        format!(
            "I found {modality} at {timestamp}: “{excerpt}”. Open the result to check the original."
        )
    };
    if result["match_kind"] == "exact" {
        if let Some(clue) = result["context_evidence"]
            .as_array()
            .and_then(|items| items.iter().find(|item| item["modality"] == "visual_label"))
            .and_then(|item| item["text"].as_str())
        {
            let clue_excerpt: String = clue.chars().take(120).collect();
            answer.push_str(&format!(
                " A nearby visual suggestion also mentions “{clue_excerpt}”; it does not identify who is speaking."
            ));
        }
    }
    Some(answer)
}

fn finish(
    mut matches: Vec<Value>,
    total: usize,
    indexed_count: usize,
    audio_count: usize,
    frame_count: usize,
    sound_event_count: usize,
) -> Value {
    matches.sort_by(|a, b| {
        b["score"]
            .as_u64()
            .cmp(&a["score"].as_u64())
            .then_with(|| a["start_ms"].as_u64().cmp(&b["start_ms"].as_u64()))
    });
    let mut unique = Vec::<Value>::new();
    for result in matches {
        let duplicate = unique.iter().any(|existing| {
            existing["asset_id"] == result["asset_id"]
                && existing["start_ms"]
                    .as_u64()
                    .unwrap_or(0)
                    .abs_diff(result["start_ms"].as_u64().unwrap_or(0))
                    <= NEARBY_MS
        });
        if !duplicate {
            unique.push(result);
            if unique.len() == 20 {
                break;
            }
        }
    }
    let answer = if unique.is_empty() {
        if total == 0 {
            "This Room has no videos to search yet. Add a video to search spoken words and on-screen text."
                .to_owned()
        } else {
            format!(
                "I couldn't find a matching indexed moment. {indexed_count} of {total} videos are indexed here; sampled frames and available audio can still miss details."
            )
        }
    } else {
        evidence_answer(&unique[0]).unwrap_or_else(|| {
            format!(
                "I found {} matching video moment{}. Open result to check its time-stamped evidence.",
                unique.len(),
                if unique.len() == 1 { "" } else { "s" }
            )
        })
    };
    json!({
        "answer":answer,"matches":unique,
        "coverage":{"videos":total,"indexed":indexed_count,
        "audio_indexed":audio_count,"frames_indexed":frame_count,
        "sound_events_indexed":sound_event_count}
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speech_index_exports_ordered_safe_webvtt_without_visual_classifier_labels() {
        let index = VideoIndex {
            room_id: "room".into(),
            asset_id: "movie".into(),
            asset_sha256: "hash".into(),
            audio_indexed: true,
            sound_events_indexed: false,
            frames_indexed: true,
            visual_labels_indexed: true,
            frame_interval_ms: Some(2_000),
            observations: vec![
                VideoObservation {
                    modality: "speech".into(),
                    start_ms: 10_000,
                    end_ms: 11_500,
                    text: "later".into(),
                    confidence_percent: 90,
                },
                VideoObservation {
                    modality: "visual_label".into(),
                    start_ms: 500,
                    end_ms: 500,
                    text: "birthday cake".into(),
                    confidence_percent: 80,
                },
                VideoObservation {
                    modality: "speech".into(),
                    start_ms: 1_000,
                    end_ms: 1_000,
                    text: "hello <family> & friends --> today".into(),
                    confidence_percent: 92,
                },
            ],
        };

        let (webvtt, cues) = speech_webvtt(&index);
        assert_eq!(cues.len(), 2);
        assert_eq!(
            webvtt,
            "WEBVTT\n\n1\n00:00:01.000 --> 00:00:01.001\nhello &lt;family&gt; &amp; friends --&gt; today\n\n2\n00:00:10.000 --> 00:00:11.500\nlater\n\n"
        );
    }

    #[test]
    fn older_local_indexes_default_to_no_sound_event_coverage() {
        let index: VideoIndex = serde_json::from_value(json!({
            "room_id":"room", "asset_id":"movie", "asset_sha256":"hash",
            "audio_indexed":true, "frames_indexed":false,
            "visual_labels_indexed":false, "frame_interval_ms":null,
            "observations":[]
        }))
        .unwrap();

        assert!(!index.sound_events_indexed);
    }

    #[test]
    fn bounded_postings_fall_back_when_evidence_exceeds_memory_budget() {
        let entry = VideoObservation {
            modality: "speech".into(),
            start_ms: 0,
            end_ms: 100,
            text: "a remembered sentence".into(),
            confidence_percent: 90,
        };
        let index = VideoIndex {
            room_id: "room".into(),
            asset_id: "movie".into(),
            asset_sha256: "hash".into(),
            audio_indexed: true,
            sound_events_indexed: false,
            frames_indexed: false,
            visual_labels_indexed: false,
            frame_interval_ms: None,
            observations: vec![entry.clone(), entry],
        };
        assert!(VideoSearchCache::build_limited(&[&index], 2, 100).is_some());
        assert!(VideoSearchCache::build_limited(&[&index], 1, 100).is_none());
        assert!(VideoSearchCache::build_limited(&[&index], 2, 10).is_none());
    }

    #[test]
    fn empty_video_search_explains_how_to_start() {
        let empty_room = search(&[], &[], "birthday", "all");
        assert_eq!(
            empty_room["answer"],
            "This Room has no videos to search yet. Add a video to search spoken words and on-screen text."
        );
        assert_eq!(empty_room["coverage"]["videos"], 0);

        let no_local_index = finish(Vec::new(), 2, 0, 0, 0, 0);
        assert!(no_local_index["answer"]
            .as_str()
            .unwrap()
            .contains("0 of 2 videos are indexed here"));
    }

    #[test]
    fn conversational_contractions_do_not_block_video_evidence_search() {
        let frame_question = search_query("What's written on the sign?", None);
        assert!(frame_question.browse_evidence);
        assert_eq!(frame_question.browse_modality, Some("frame"));

        let spoken_question = search_query("Where's Grandma's blue balloon?", None);
        let terms: Vec<_> = tokens(&spoken_question.text)
            .into_iter()
            .filter(|term| !STOP_WORDS.contains(&term.as_str()))
            .collect();
        assert_eq!(terms, ["grandma", "blue", "balloon"]);
    }

    #[test]
    fn suggestion_terms_remove_stop_words_before_normalizing() {
        let query = search_query("Find this kid blowing candles", None);
        assert_eq!(normalized_query_terms(&query), ["blow", "candle", "child"]);
        assert_eq!(searchable_token("making"), "make");
        assert_eq!(searchable_token("makes"), "make");
        assert_eq!(searchable_token("running"), "run");
        assert_eq!(searchable_token("runs"), "run");
    }

    fn indexed_video(modality: &str, text: &str) -> (VideoIndex, Vec<MediaAsset>) {
        let asset_id = "movie".to_owned();
        let room_id = "room".to_owned();
        (
            VideoIndex {
                room_id: room_id.clone(),
                asset_id: asset_id.clone(),
                asset_sha256: "verified-hash".into(),
                audio_indexed: modality == "speech" || modality == "sound_event",
                sound_events_indexed: modality == "sound_event",
                frames_indexed: modality == "screen_text" || modality == "visual_label",
                visual_labels_indexed: modality == "visual_label",
                frame_interval_ms: Some(2_000),
                observations: vec![VideoObservation {
                    modality: modality.into(),
                    start_ms: 12_000,
                    end_ms: 14_000,
                    text: text.into(),
                    confidence_percent: 80,
                }],
            },
            vec![MediaAsset {
                id: asset_id,
                room_id,
                filename: "family-moment.mp4".into(),
                kind: "video".into(),
                sha256: "verified-hash".into(),
                size: 1_024,
                captured_at: 0,
                contributed_at: 0,
                contributor: "device".into(),
                favorite: false,
                caption: String::new(),
                deleted: false,
                people: Vec::new(),
                components: Vec::new(),
                key_epoch: 0,
            }],
        )
    }

    #[test]
    fn concept_search_finds_a_visual_paraphrase_as_a_suggestion_in_both_retrievers() {
        let (index, assets) = indexed_video("visual_label", "An audience applauding");
        let indexes = [&index];
        let cache = VideoSearchCache::build(&indexes).unwrap();
        let query = "Find a crowd clapping";
        let cached = cache.search(&indexes, &assets, query, "frame");
        let fallback = search(&indexes, &assets, query, "frame");

        assert_eq!(cached, fallback, "cached and fallback retrieval must agree");
        assert_eq!(cached["matches"].as_array().unwrap().len(), 1);
        assert_eq!(cached["matches"][0]["start_ms"], 12_000);
        assert_eq!(cached["matches"][0]["match_kind"], "suggestion");
        assert!(cached["answer"]
            .as_str()
            .unwrap()
            .contains("No exact match"));
    }

    #[test]
    fn one_word_sound_synonyms_are_findable_but_never_claimed_as_exact() {
        let (index, assets) = indexed_video("sound_event", "applause");
        let indexes = [&index];
        let response = search(&indexes, &assets, "clapping", "audio");

        assert_eq!(response["matches"].as_array().unwrap().len(), 1);
        assert_eq!(response["matches"][0]["match_kind"], "suggestion");
        assert_eq!(
            response["matches"][0]["evidence"][0]["modality"],
            "sound_event"
        );
    }

    #[test]
    fn quoted_speech_and_frame_phrases_do_not_expand_to_semantic_synonyms() {
        for (modality, evidence, query, filter) in [
            (
                "speech",
                "Everyone applauded",
                "‘Everyone clapped’",
                "audio",
            ),
            (
                "screen_text",
                "Audience applauding",
                "‘Crowd clapping’",
                "frame",
            ),
        ] {
            let (index, assets) = indexed_video(modality, evidence);
            let indexes = [&index];
            let response = search(&indexes, &assets, query, filter);

            assert!(
                response["matches"].as_array().unwrap().is_empty(),
                "quoted phrase {query} must not match aliased {evidence}"
            );
        }
    }

    #[test]
    fn quoted_speech_phrase_uses_nearby_frame_context_to_rank_exact_moments() {
        let (mut index, assets) = indexed_video("speech", "unused fixture text");
        index.frames_indexed = true;
        index.visual_labels_indexed = true;
        index.observations = vec![
            VideoObservation {
                modality: "speech".into(),
                start_ms: 10_000,
                end_ms: 11_500,
                text: "Happy birthday".into(),
                confidence_percent: 92,
            },
            VideoObservation {
                modality: "visual_label".into(),
                start_ms: 10_500,
                end_ms: 12_000,
                text: "A child blowing out candles".into(),
                confidence_percent: 65,
            },
            VideoObservation {
                modality: "speech".into(),
                start_ms: 50_000,
                end_ms: 51_500,
                text: "Happy birthday".into(),
                confidence_percent: 92,
            },
            VideoObservation {
                modality: "visual_label".into(),
                start_ms: 50_500,
                end_ms: 52_000,
                text: "A child opening presents".into(),
                confidence_percent: 65,
            },
        ];
        let indexes = [&index];
        let cache = VideoSearchCache::build(&indexes).unwrap();
        let query = "Find the moment with ‘Happy birthday’ while blowing candles";

        let cached = cache.search(&indexes, &assets, query, "all");
        let fallback = search(&indexes, &assets, query, "all");

        assert_eq!(cached, fallback, "cached and fallback retrieval must agree");
        let matches = cached["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0]["start_ms"], 10_000);
        assert_eq!(matches[0]["match_kind"], "exact");
        assert_eq!(matches[0]["context_evidence"].as_array().unwrap().len(), 1);
        assert_eq!(
            matches[0]["context_evidence"][0]["modality"],
            "visual_label"
        );
        assert_eq!(matches[1]["start_ms"], 50_000);
        assert_eq!(matches[1]["match_kind"], "exact");
        assert!(matches[1]["context_evidence"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(
            cached["answer"]
                .as_str()
                .unwrap()
                .contains("nearby visual suggestion"),
            "unexpected mixed-evidence answer: {}",
            cached["answer"]
        );
    }

    #[test]
    fn quoted_phrase_cannot_be_assembled_from_audio_and_frame_fragments() {
        let (mut index, assets) = indexed_video("speech", "unused fixture text");
        index.frames_indexed = true;
        index.observations = vec![
            VideoObservation {
                modality: "speech".into(),
                start_ms: 10_000,
                end_ms: 10_500,
                text: "Happy".into(),
                confidence_percent: 90,
            },
            VideoObservation {
                modality: "screen_text".into(),
                start_ms: 10_000,
                end_ms: 10_500,
                text: "birthday".into(),
                confidence_percent: 90,
            },
        ];

        let result = search(&[&index], &assets, "‘Happy birthday’", "all");

        assert!(result["matches"].as_array().unwrap().is_empty());
    }

    #[test]
    fn quoted_scene_labels_remain_suggestions_without_transcript_or_ocr_evidence() {
        let (index, assets) = indexed_video("visual_label", "Child blowing candles");

        let result = search(&[&index], &assets, "‘Child blowing candles’", "frame");

        assert_eq!(result["matches"][0]["match_kind"], "suggestion");
        assert!(result["answer"]
            .as_str()
            .unwrap()
            .contains("No exact match"));
    }

    #[test]
    fn follow_up_video_questions_reuse_local_context_without_claiming_exactness() {
        let follow_up = search_query(
            "What about the candles?",
            Some("Find the birthday video where Grandma says ‘yellow balloon’"),
        );
        assert!(follow_up.contextual);
        assert!(!follow_up.quoted);
        assert!(!follow_up.browse_evidence);
        assert!(follow_up.text.contains("candles"));
        assert!(follow_up.context_text.contains("yellow balloon"));

        let exact = search_query(
            "What does she say when she says ‘blow them out’?",
            Some("Find the birthday video where Grandma says ‘yellow balloon’"),
        );
        assert!(!exact.contextual);
        assert!(exact.quoted);
        assert_eq!(exact.text, "blow them out");

        let independent = search_query("Find a dog on the beach", Some("Find the birthday video"));
        assert!(!independent.contextual);
        assert!(!independent.text.contains("birthday"));
    }
}
