//! The widget's user-facing strings, set once per frame via [`Cabinet::labels`].

use crate::Cabinet;

/// The status lines and the selection count line. English by default; localise either by field ([`Labels { loading: "Lädt…".into(), ..Labels::german() }`](Labels)) or wholesale ([`german`](Self::german)). The column headers are not here — they belong to the [columns](crate::Column) themselves.
pub struct Labels<'a> {
    /// Note in place of a directory whose listing is on its way. Default: `Loading…`.
    pub loading: String,
    /// Note in place of an empty listing. Default: `Empty`.
    pub empty: String,
    /// Note under a failed listing; tapping it retries. Default: `Could not load — tap to retry`.
    pub retry: String,
    /// The header line while entries are selected, called with the count. Default: `2 selected`.
    pub selected: Box<dyn Fn(usize) -> String + 'a>,
}

impl Default for Labels<'_> {
    fn default() -> Self {
        Self {
            loading: "Loading…".into(),
            empty: "Empty".into(),
            retry: "Could not load — tap to retry".into(),
            selected: Box::new(|count| format!("{count} selected")),
        }
    }
}

impl<'a> Labels<'a> {
    /// German strings for every line, including the count.
    pub fn german() -> Self {
        Self {
            loading: "Lädt…".into(),
            empty: "Leer".into(),
            retry: "Konnte nicht laden — erneut versuchen".into(),
            selected: Box::new(|count| format!("{count} ausgewählt")),
        }
    }
}

impl<'a, Message: Clone, Renderer> Cabinet<'a, Message, Renderer> {
    /// Localise the status and count lines for this frame. Default: English [`Labels::default`].
    #[must_use]
    pub fn labels(mut self, labels: Labels<'a>) -> Self {
        self.labels = labels;
        self
    }
}
