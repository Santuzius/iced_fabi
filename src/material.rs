//! The Material Icon Theme as [`RowIcons`] (feature `material-icon-theme`): a vendored cut of its SVGs read into icondata glyphs, files looked up by extension.

use std::{collections::HashMap, sync::LazyLock};

use crate::{Node, RowIcons};

/// Turn one vendored SVG's static file contents into icondata path data: the file's own `viewBox` and everything between the outer tags. The widget wraps that in a document with no fill or stroke of its own, so every path keeps the colours the theme authored.
fn svg_glyph(svg: &'static str) -> icondata_core::IconData {
    let view_box = match svg.find(r#"viewBox=""#) {
        Some(start) => {
            let rest = &svg[start + 9..];
            &rest[..rest.find('"').unwrap()]
        }
        None => "0 0 24 24",
    };
    let inner = &svg[svg.find('>').unwrap() + 1..svg.rfind("</svg>").unwrap()];
    icondata_core::IconData {
        style: None,
        x: None,
        y: None,
        width: None,
        height: None,
        view_box: Some(view_box),
        stroke_linecap: None,
        stroke_linejoin: None,
        stroke_width: None,
        stroke: Some("none"),
        fill: Some("none"),
        data: inner,
    }
}

/// File extensions (lower case) and the theme's glyph for them, following the theme's own `fileExtensions` table.
const BY_EXTENSION: &[(&str, &str)] = &[
    ("md", include_str!("../assets/material-icon-theme/markdown.svg")),
    ("pdf", include_str!("../assets/material-icon-theme/pdf.svg")),
    ("doc", include_str!("../assets/material-icon-theme/word.svg")),
    ("docx", include_str!("../assets/material-icon-theme/word.svg")),
    ("xls", include_str!("../assets/material-icon-theme/table.svg")),
    ("xlsx", include_str!("../assets/material-icon-theme/table.svg")),
    ("ppt", include_str!("../assets/material-icon-theme/powerpoint.svg")),
    ("pptx", include_str!("../assets/material-icon-theme/powerpoint.svg")),
    ("json", include_str!("../assets/material-icon-theme/json.svg")),
    ("js", include_str!("../assets/material-icon-theme/javascript.svg")),
    ("ts", include_str!("../assets/material-icon-theme/typescript.svg")),
    ("py", include_str!("../assets/material-icon-theme/python.svg")),
    ("rs", include_str!("../assets/material-icon-theme/rust.svg")),
    ("css", include_str!("../assets/material-icon-theme/css.svg")),
    ("html", include_str!("../assets/material-icon-theme/html.svg")),
    ("zip", include_str!("../assets/material-icon-theme/zip.svg")),
    ("png", include_str!("../assets/material-icon-theme/image.svg")),
    ("mp4", include_str!("../assets/material-icon-theme/video.svg")),
    ("mp3", include_str!("../assets/material-icon-theme/audio.svg")),
    ("svg", include_str!("../assets/material-icon-theme/svg.svg")),
    ("sh", include_str!("../assets/material-icon-theme/console.svg")),
    ("lock", include_str!("../assets/material-icon-theme/lock.svg")),
    ("toml", include_str!("../assets/material-icon-theme/toml.svg")),
    ("yaml", include_str!("../assets/material-icon-theme/yaml.svg")),
    ("yml", include_str!("../assets/material-icon-theme/yaml.svg")),
    ("xml", include_str!("../assets/material-icon-theme/xml.svg")),
    ("nix", include_str!("../assets/material-icon-theme/nix.svg")),
    ("go", include_str!("../assets/material-icon-theme/go.svg")),
    ("java", include_str!("../assets/material-icon-theme/java.svg")),
    ("txt", include_str!("../assets/material-icon-theme/document.svg")),
    ("7z", include_str!("../assets/material-icon-theme/zip.svg")),
    ("rar", include_str!("../assets/material-icon-theme/zip.svg")),
];

struct Glyphs {
    folder: icondata_core::IconData,
    folder_open: icondata_core::IconData,
    file: icondata_core::IconData,
    by_extension: HashMap<&'static str, icondata_core::IconData>,
}

/// A cut of the Material Icon Theme for VS Code (© Material Extensions, MIT — the license text sits next to the vendored files, their provenance in the root NOTICE).
static GLYPHS: LazyLock<Glyphs> = LazyLock::new(|| Glyphs {
    folder: svg_glyph(include_str!("../assets/material-icon-theme/folder.svg")),
    folder_open: svg_glyph(include_str!("../assets/material-icon-theme/folder-open.svg")),
    file: svg_glyph(include_str!("../assets/material-icon-theme/file.svg")),
    by_extension: BY_EXTENSION
        .iter()
        .map(|&(extension, svg)| (extension, svg_glyph(svg)))
        .collect(),
});

/// The extension of `name` in lower case: the part after the last dot, if a stem comes before it and it is at most five characters long.
fn extension(name: &str) -> Option<String> {
    let (stem, extension) = name.rsplit_once('.')?;
    (!stem.is_empty() && extension.len() <= 5).then(|| extension.to_ascii_lowercase())
}

/// [`RowIcons`] from the Material Icon Theme for VS Code: its blue-grey folder and open folder, and a file glyph per extension (about 30 common formats, a plain page for the rest). The glyphs keep their own colours, so [`Style::folder`](crate::Style::folder) does not apply. Pass it to [`Cabinet::icons`](crate::Cabinet::icons).
pub struct MaterialIconTheme;

impl RowIcons for MaterialIconTheme {
    fn folder(&self, _node: &Node, open: bool) -> icondata::Icon {
        if open {
            &GLYPHS.folder_open
        } else {
            &GLYPHS.folder
        }
    }

    fn file(&self, node: &Node) -> icondata::Icon {
        extension(&node.name)
            .and_then(|extension| GLYPHS.by_extension.get(extension.as_str()))
            .unwrap_or(&GLYPHS.file)
    }

    fn tinted(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_glyph_has_a_view_box_and_paths() {
        for glyph in [&GLYPHS.folder, &GLYPHS.folder_open, &GLYPHS.file].into_iter().chain(GLYPHS.by_extension.values()) {
            assert!(glyph.view_box.is_some_and(|v| !v.is_empty()));
            assert!(glyph.data.contains("<path"));
        }
    }

    #[test]
    fn extensions_are_matched_without_case() {
        assert_eq!(extension("Scan.PDF").as_deref(), Some("pdf"));
        assert_eq!(extension(".bashrc"), None);
        assert_eq!(extension("archive.tar.gz").as_deref(), Some("gz"));
    }
}
