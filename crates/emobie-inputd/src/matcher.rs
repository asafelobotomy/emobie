//! Trigger trie for as-you-type expansion (immediate or after Space).

use crate::protocol::TriggerMode;

#[derive(Default)]
struct Node {
    children: std::collections::HashMap<char, Node>,
    expansion: Option<(String, TriggerMode)>,
}

#[derive(Default)]
pub struct TriggerTrie {
    root: Node,
}

impl TriggerTrie {
    pub fn clear(&mut self) {
        self.root = Node::default();
    }

    pub fn insert(&mut self, trigger: &str, expansion: &str, mode: TriggerMode) {
        if trigger.is_empty() || expansion.is_empty() {
            return;
        }
        let mut node = &mut self.root;
        for ch in trigger.chars() {
            node = node.children.entry(ch).or_default();
        }
        node.expansion = Some((expansion.to_string(), mode));
    }

    pub fn load(&mut self, matches: &[(String, String, TriggerMode)]) {
        self.clear();
        for (trigger, expansion, mode) in matches {
            self.insert(trigger, expansion, *mode);
        }
    }

    /// Match against `buffer`.
    /// - `immediate`: fires as soon as the trigger is a suffix.
    /// - `space`: fires only when the buffer ends with `trigger` + Space; the
    ///   Space is consumed and re-appended after the expansion.
    pub fn match_suffix(&self, buffer: &str) -> Option<(usize, String)> {
        let chars: Vec<char> = buffer.chars().collect();
        if chars.is_empty() {
            return None;
        }
        let ends_with_space = chars.last() == Some(&' ');
        let mut best: Option<(usize, String)> = None;

        if ends_with_space && chars.len() >= 2 {
            let without_space: String = chars[..chars.len() - 1].iter().collect();
            if let Some((len, expansion, mode)) = self.best_terminal(&without_space) {
                if mode == TriggerMode::Space {
                    let erase = len + 1;
                    best = Some((erase, format!("{expansion} ")));
                }
            }
        }

        if let Some((len, expansion, mode)) = self.best_terminal(buffer) {
            if mode == TriggerMode::Immediate {
                let candidate = (len, expansion);
                best = match best {
                    Some(prev) if prev.0 >= candidate.0 => Some(prev),
                    _ => Some(candidate),
                };
            }
        }

        best
    }

    fn best_terminal(&self, buffer: &str) -> Option<(usize, String, TriggerMode)> {
        let chars: Vec<char> = buffer.chars().collect();
        if chars.is_empty() {
            return None;
        }
        let mut best: Option<(usize, String, TriggerMode)> = None;
        for start in 0..chars.len() {
            let mut node = &self.root;
            let mut matched = true;
            for ch in chars.iter().skip(start) {
                match node.children.get(ch) {
                    Some(next) => node = next,
                    None => {
                        matched = false;
                        break;
                    }
                }
            }
            if matched {
                if let Some((expansion, mode)) = &node.expansion {
                    let len = chars.len() - start;
                    let candidate = (len, expansion.clone(), *mode);
                    best = match best {
                        Some(prev) if prev.0 >= candidate.0 => Some(prev),
                        _ => Some(candidate),
                    };
                }
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_longest_immediate_suffix() {
        let mut trie = TriggerTrie::default();
        trie.insert(":s", "short", TriggerMode::Immediate);
        trie.insert(":sig", "signature", TriggerMode::Immediate);
        assert_eq!(
            trie.match_suffix("hello:sig"),
            Some((4, "signature".into()))
        );
        assert_eq!(trie.match_suffix("x:s"), Some((2, "short".into())));
        assert_eq!(trie.match_suffix("nope"), None);
    }

    #[test]
    fn space_mode_waits_for_space() {
        let mut trie = TriggerTrie::default();
        trie.insert(":sig", "signature", TriggerMode::Space);
        assert_eq!(trie.match_suffix("hello:sig"), None);
        assert_eq!(
            trie.match_suffix("hello:sig "),
            Some((5, "signature ".into()))
        );
    }

    #[test]
    fn immediate_still_fires_before_space() {
        let mut trie = TriggerTrie::default();
        trie.insert(":)", "🙂", TriggerMode::Immediate);
        assert_eq!(trie.match_suffix("hi:)"), Some((2, "🙂".into())));
    }
}
