//! Natural ("File 2" < "File 10") comparison keys for names, the fallback order of every column sort.

use std::cmp::Ordering;

/// Natural-order key for names: runs of digits compare by value, so `File 10` sorts after `File 9`.
/// Names of any length and digit runs of any width are accepted; nothing panics.
pub(crate) fn name_key(name: &str) -> Vec<NameChunk> {
    let mut chunks = Vec::new();
    let mut digits = String::new();
    for ch in name.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            if !digits.is_empty() {
                chunks.push(number_chunk(std::mem::take(&mut digits)));
            }
            chunks.push(NameChunk::Text(ch.to_lowercase().collect()));
        }
    }
    if !digits.is_empty() {
        chunks.push(number_chunk(digits));
    }
    chunks
}

/// One digit run as a [`NameChunk::Number`]: value digits without leading zeros, plus the run's original width.
fn number_chunk(run: String) -> NameChunk {
    let width = run.len();
    NameChunk::Number {
        digits: run.trim_start_matches('0').to_owned(),
        width,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum NameChunk {
    Text(String),
    /// A digit run: `digits` are the run's digits with leading zeros stripped (empty for an all-zero run), `width` is the original run's length.
    Number {
        digits: String,
        width: usize,
    },
}

impl Ord for NameChunk {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (NameChunk::Text(a), NameChunk::Text(b)) => a.cmp(b),
            // Digit runs compare by value without parsing: more significant digits mean a larger value, and for equally many the digit strings compare lexicographically. Equal values (7 and 007) then order by the original run's length, so names that differ only in zero padding keep a stable order.
            (
                NameChunk::Number {
                    digits: a,
                    width: aw,
                },
                NameChunk::Number {
                    digits: b,
                    width: bw,
                },
            ) => a
                .len()
                .cmp(&b.len())
                .then_with(|| a.cmp(b))
                .then_with(|| aw.cmp(bw)),
            (NameChunk::Text(_), NameChunk::Number { .. }) => Ordering::Less,
            (NameChunk::Number { .. }, NameChunk::Text(_)) => Ordering::Greater,
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

    #[test]
    fn digit_runs_wider_than_u64_do_not_panic() {
        let key = name_key("IMG_123456789012345678901234.jpg");
        assert_eq!(key.len(), 9); // IMG_ + one digit run + .jpg
        assert!(key > name_key("IMG_9.jpg"));
        assert!(name_key("a99999999999999999999999") > name_key("a9"));
    }

    #[test]
    fn equal_digit_values_order_stably_by_their_original_length() {
        assert!(name_key("a7") < name_key("a007"));
        assert!(name_key("a007") < name_key("a0007"));
        assert!(name_key("a0") < name_key("a00"));
    }
}
