//! Natural ("File 2" < "File 10") comparison keys for names, the fallback order of every column sort.

use std::cmp::Ordering;

/// Natural-order key for names: runs of digits compare by value, so `File 10` sorts after `File 9`.
pub(crate) fn name_key(name: &str) -> Vec<NameChunk> {
    let mut chunks = Vec::new();
    let mut digits = String::new();
    for ch in name.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            if !digits.is_empty() {
                chunks.push(NameChunk::Number(
                    digits.parse().expect("digits parse as u64"),
                ));
                digits.clear();
            }
            chunks.push(NameChunk::Text(ch.to_lowercase().collect()));
        }
    }
    if !digits.is_empty() {
        chunks.push(NameChunk::Number(
            digits.parse().expect("digits parse as u64"),
        ));
    }
    chunks
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum NameChunk {
    Text(String),
    Number(u64),
}

impl Ord for NameChunk {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (NameChunk::Text(a), NameChunk::Text(b)) => a.cmp(b),
            (NameChunk::Number(a), NameChunk::Number(b)) => a.cmp(b),
            (NameChunk::Text(_), NameChunk::Number(_)) => Ordering::Less,
            (NameChunk::Number(_), NameChunk::Text(_)) => Ordering::Greater,
        }
    }
}

impl PartialOrd for NameChunk {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::name_key;

    #[test]
    fn natural_name_sort() {
        assert!(name_key("File 9") < name_key("File 10"));
        assert!(name_key("apple") < name_key("Banana"));
    }
}
