//! The widget's user-facing strings, set once per frame via [`Cabinet::labels`].

use crate::Cabinet;

/// The status lines, the selection count line and the new folder dialog. English by default; localise either by field ([`Labels { loading: "Lädt…".into(), ..Labels::german() }`](Labels)) or wholesale ([`german`](Self::german)). The column headers are not here — they belong to the [columns](crate::Column) themselves.
pub struct Labels<'a> {
    /// Note in place of a directory whose listing is on its way. Default: `Loading…`.
    pub loading: String,
    /// Note in place of an empty listing. Default: `Empty`.
    pub empty: String,
    /// Note under a failed listing; tapping it retries. Default: `Could not load — tap to retry`.
    pub retry: String,
    /// The header line while entries are selected, called with the count. Default: `2 selected`.
    pub selected: Box<dyn Fn(usize) -> String + 'a>,
    /// Title and drafted name of the new folder dialog. Default: `New folder`.
    pub new_folder: String,
    /// The question under the new folder dialog's heading, asked to the user. Default: `What should the new folder be called?`.
    pub new_folder_question: String,
    /// The new folder dialog's confirm button. Default: `Create`.
    pub create: String,
    /// The dismiss button of the dialogs and of [`Cabinet::cancel`](crate::Cabinet::cancel). Default: `Cancel`.
    pub cancel: String,
    /// Title and confirm button of the tool confirmation modal for Delete. Default: `Delete`.
    pub delete: String,
    /// The confirmation modal's question for Delete. Default: `Are you sure you want to delete the selected entries?`.
    pub delete_question: String,
    /// The Delete confirmation's second line: what deleting the selection removes, called with (folders, files). Default: `7 folders, 18 files`, zero parts left out.
    pub delete_count: Box<dyn Fn(usize, usize) -> String + 'a>,
}

impl Default for Labels<'_> {
    fn default() -> Self {
        Self {
            loading: "Loading…".into(),
            empty: "Empty".into(),
            retry: "Could not load — tap to retry".into(),
            selected: Box::new(|count| format!("{count} selected")),
            new_folder: "New folder".into(),
            new_folder_question: "What should the new folder be called?".into(),
            create: "Create".into(),
            cancel: "Cancel".into(),
            delete: "Delete".into(),
            delete_question: "Are you sure you want to delete the selected entries?".into(),
            delete_count: Box::new(|folders, files| {
                let mut parts = Vec::new();
                if folders > 0 {
                    parts.push(format!(
                        "{} folder{}",
                        folders,
                        if folders == 1 { "" } else { "s" }
                    ));
                }
                if files > 0 {
                    parts.push(format!(
                        "{} file{}",
                        files,
                        if files == 1 { "" } else { "s" }
                    ));
                }
                parts.join(", ")
            }),
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
            new_folder: "Neuer Ordner".into(),
            new_folder_question: "Wie soll der neue Ordner heißen?".into(),
            create: "Erstellen".into(),
            cancel: "Abbrechen".into(),
            delete: "Löschen".into(),
            delete_question: "Sind Sie sicher, dass Sie die markierten Einträge löschen möchten?"
                .into(),
            delete_count: Box::new(|folders, files| {
                let mut parts = Vec::new();
                if folders > 0 {
                    parts.push(format!("{folders} Ordner"));
                }
                if files > 0 {
                    parts.push(format!(
                        "{} Datei{}",
                        files,
                        if files == 1 { "" } else { "en" }
                    ));
                }
                parts.join(", ")
            }),
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
