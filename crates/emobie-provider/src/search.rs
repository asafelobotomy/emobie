//! Search: the query is matched against names, keywords (emojibase tags and shortcodes) and
//! emoticons, as the app's search does. Favourites rank first, then recents, then the rest by
//! how well they match.

use crate::data::{Catalog, Entry};
use crate::prefs::Prefs;
use std::collections::HashMap;

/// uQ shows at most 20 rows per provider.
pub const MAX_ITEMS: usize = 20;
/// Longer queries are not emoji searches; answer nothing.
pub const MAX_QUERY_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    /// What gets copied or typed; also the row id.
    pub emoji: String,
    pub name: String,
    /// 0-1, never increasing down the list.
    pub score: f64,
}

/// The query to match, lowercased: `(raw, words)`. `words` also drops colons at either end, so
/// `:smile:` (a shortcode) and `smile` match the same; `raw` keeps them for emoticons (`:)`).
/// `None` for anything that is not a plausible search: too long or with control characters.
pub fn normalize(query: &str) -> Option<(String, String)> {
    if query.len() > MAX_QUERY_BYTES || query.chars().any(char::is_control) {
        return None;
    }
    let raw = query.trim().to_lowercase();
    let words = raw.trim_matches(':').trim().to_string();
    Some((raw, words))
}

/// How well `e` matches, 1-100; `None` when it doesn't.
fn tier(e: &Entry, raw: &str, words: &str) -> Option<u8> {
    let mut best = 0u8;
    let mut up = |t: u8| best = best.max(t);
    if !words.is_empty() {
        if e.name_lc == words {
            up(100);
        } else if e.name_lc.starts_with(words) {
            up(85);
        } else if e.name_lc.split(' ').any(|w| w.starts_with(words)) {
            up(70);
        } else if e.name_lc.contains(words) {
            up(40);
        }
        for s in &e.shortcodes_lc {
            if s == words {
                up(90);
            } else if s.starts_with(words) {
                up(60);
            } else if s.contains(words) {
                up(30);
            }
        }
        for k in &e.keywords_lc {
            if k == words {
                up(75);
            } else if k.starts_with(words) {
                up(50);
            } else if k.contains(words) {
                up(30);
            }
        }
    }
    if !raw.is_empty() {
        for t in &e.emoticons_lc {
            if t == raw {
                up(95);
            } else if t.contains(raw) {
                up(20);
            }
        }
    }
    (best > 0).then_some(best)
}

struct Ranked<'a> {
    /// 0 favourite, 1 recent, 2 anything else.
    class: u8,
    tier: u8,
    /// Position in favourites or recents.
    pos: usize,
    order: u32,
    emoji: &'a str,
    name: &'a str,
}

fn base_score(class: u8, tier: u8) -> f64 {
    match class {
        0 => 0.9 + f64::from(tier) / 1000.0,
        1 => 0.8 + f64::from(tier) / 1000.0,
        _ => 0.3 + f64::from(tier) / 200.0,
    }
}

/// Up to `MAX_ITEMS` hits for `query`. An empty query lists favourites, then recents.
pub fn search(catalog: &Catalog, prefs: &Prefs, query: &str) -> Vec<Hit> {
    let Some((raw, words)) = normalize(query) else {
        return Vec::new();
    };
    let empty = words.is_empty() && raw.is_empty();

    let mut favourite: HashMap<usize, usize> = HashMap::new();
    for (pos, hex) in prefs.favorites.iter().enumerate() {
        if let Some(i) = catalog.by_hexcode(hex) {
            favourite.entry(i).or_insert(pos);
        }
    }
    // Entry → (position, the variant the user copied).
    let mut recent: HashMap<usize, (usize, &str)> = HashMap::new();
    for (pos, emoji) in prefs.recents.iter().enumerate() {
        if let Some(i) = catalog.by_char(emoji) {
            recent.entry(i).or_insert((pos, emoji.as_str()));
        }
    }

    let mut ranked: Vec<Ranked> = catalog
        .entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let tier = if empty { 1 } else { tier(e, &raw, &words)? };
            let (class, pos, emoji) = if let Some(&pos) = favourite.get(&i) {
                (0, pos, e.with_tone(prefs.skin_tone))
            } else if let Some(&(pos, copied)) = recent.get(&i) {
                (1, pos, copied)
            } else if empty {
                return None;
            } else {
                (2, 0, e.with_tone(prefs.skin_tone))
            };
            Some(Ranked {
                class,
                tier,
                pos,
                order: e.order,
                emoji,
                name: &e.name,
            })
        })
        .collect();
    ranked.sort_by(|a, b| {
        (a.class, std::cmp::Reverse(a.tier), a.pos, a.order).cmp(&(
            b.class,
            std::cmp::Reverse(b.tier),
            b.pos,
            b.order,
        ))
    });

    let mut last = 1.0f64;
    ranked
        .into_iter()
        .take(MAX_ITEMS)
        .map(|r| {
            last = last.min(base_score(r.class, r.tier));
            Hit {
                emoji: r.emoji.to_string(),
                name: r.name.to_string(),
                score: last,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::tests::fixture;

    fn emojis(hits: &[Hit]) -> Vec<&str> {
        hits.iter().map(|h| h.emoji.as_str()).collect()
    }

    #[test]
    fn matches_names_keywords_shortcodes_and_emoticons() {
        let c = fixture();
        let none = Prefs::default();
        assert_eq!(emojis(&search(&c, &none, "cat")), ["🐱"]);
        assert_eq!(emojis(&search(&c, &none, "hello")), ["👋"], "keyword");
        assert_eq!(
            emojis(&search(&c, &none, "slight_smile")),
            ["🙂"],
            "shortcode"
        );
        assert_eq!(emojis(&search(&c, &none, ":)")), ["🙂"], "emoticon");
        assert_eq!(
            emojis(&search(&c, &none, ":cat:")),
            ["🐱"],
            "colons around a shortcode"
        );
        assert_eq!(
            emojis(&search(&c, &none, "  CAT ")),
            ["🐱"],
            "case and spaces"
        );
        assert!(search(&c, &none, "zebra").is_empty());
    }

    #[test]
    fn ranks_by_match_quality_then_order() {
        let c = fixture();
        let hits = search(&c, &Prefs::default(), "smile");
        // `smile` is 😄's shortcode; 😀 and 🙂 only have it as a keyword, so emoji order
        // decides between them.
        assert_eq!(emojis(&hits), ["😄", "😀", "🙂"]);
        assert!(hits.windows(2).all(|w| w[0].score >= w[1].score));
        assert_eq!(hits[0].name, "grinning face with smiling eyes");
    }

    #[test]
    fn favourites_then_recents_rank_first() {
        let c = fixture();
        let prefs = Prefs {
            favorites: vec!["1F642".into()],
            recents: vec!["😀".into(), "🙂".into()],
            skin_tone: 0,
        };
        let hits = search(&c, &prefs, "smile");
        assert_eq!(emojis(&hits), ["🙂", "😀", "😄"]);
        assert!(hits[0].score > hits[1].score && hits[1].score > hits[2].score);
        assert!(hits.iter().all(|h| (0.0..=1.0).contains(&h.score)));
    }

    #[test]
    fn empty_query_lists_favourites_then_recents() {
        let c = fixture();
        let prefs = Prefs {
            favorites: vec!["1F431".into(), "nope".into()],
            recents: vec!["👋🏽".into(), "🐱".into()],
            skin_tone: 0,
        };
        assert_eq!(emojis(&search(&c, &prefs, "")), ["🐱", "👋🏽"]);
        assert!(search(&c, &Prefs::default(), "").is_empty());
    }

    #[test]
    fn applies_skin_tone_and_keeps_the_copied_variant() {
        let c = fixture();
        let tone = Prefs {
            skin_tone: 1,
            ..Prefs::default()
        };
        assert_eq!(emojis(&search(&c, &tone, "wave")), ["👋🏻"]);
        let recent = Prefs {
            recents: vec!["👋🏽".into()],
            skin_tone: 1,
            ..Prefs::default()
        };
        assert_eq!(emojis(&search(&c, &recent, "wave")), ["👋🏽"]);
    }

    #[test]
    fn rejects_hostile_queries_and_caps_results() {
        let c = fixture();
        let none = Prefs::default();
        assert!(search(&c, &none, "cat\u{0}").is_empty());
        assert!(search(&c, &none, "cat\n").is_empty());
        assert!(search(&c, &none, &"a".repeat(MAX_QUERY_BYTES + 1)).is_empty());
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/data/emoji.json");
        let real = Catalog::load(&path).unwrap();
        let hits = search(&real, &none, "face");
        assert_eq!(hits.len(), MAX_ITEMS);
        assert_eq!(search(&real, &none, "thumbs up")[0].emoji, "👍\u{fe0f}");
    }
}
