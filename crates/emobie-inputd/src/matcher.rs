//! Simple trigger trie for as-you-type expansion.

#[derive(Default)]
struct Node {
    children: std::collections::HashMap<char, Node>,
    expansion: Option<String>,
}

#[derive(Default)]
pub struct TriggerTrie {
    root: Node,
}

impl TriggerTrie {
    pub fn clear(&mut self) {
        self.root = Node::default();
    }

    pub fn insert(&mut self, trigger: &str, expansion: &str) {
        if trigger.is_empty() || expansion.is_empty() {
            return;
        }
        let mut node = &mut self.root;
        for ch in trigger.chars() {
            node = node.children.entry(ch).or_default();
        }
        node.expansion = Some(expansion.to_string());
    }

    pub fn load(&mut self, matches: &[(String, String)]) {
        self.clear();
        for (trigger, expansion) in matches {
            self.insert(trigger, expansion);
        }
    }

    /// Walk `buffer` from the end; return (trigger_len_chars, expansion) on hit.
    pub fn match_suffix(&self, buffer: &str) -> Option<(usize, String)> {
        let chars: Vec<char> = buffer.chars().collect();
        if chars.is_empty() {
            return None;
        }
        let mut best: Option<(usize, String)> = None;
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
                if let Some(expansion) = &node.expansion {
                    let len = chars.len() - start;
                    best = Some((len, expansion.clone()));
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
    fn matches_longest_suffix() {
        let mut trie = TriggerTrie::default();
        trie.insert(":s", "short");
        trie.insert(":sig", "signature");
        assert_eq!(
            trie.match_suffix("hello:sig"),
            Some((4, "signature".into()))
        );
        assert_eq!(trie.match_suffix("x:s"), Some((2, "short".into())));
        assert_eq!(trie.match_suffix("nope"), None);
    }
}
