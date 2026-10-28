//! The Material Icon Theme option: the vendored SVGs read into icondata glyphs and the [`iced_fabi::RowIcons`] implementation that maps file extensions to them.

use std::{collections::HashMap, sync::LazyLock};

use iced_fabi::Node;

use crate::extension;

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

/// A cut of the Material Icon Theme for VS Code (© Material Extensions, MIT — the license text sits next to the vendored files, their provenance in the root NOTICE).
static MATERIAL_ICONS: LazyLock<MatIcons> = LazyLock::new(|| MatIcons {
    folder: svg_glyph(include_str!("../assets/material-icon-theme/folder.svg")),
    folder_open: svg_glyph(include_str!(
        "../assets/material-icon-theme/folder-open.svg"
    )),
    file: svg_glyph(include_str!("../assets/material-icon-theme/file.svg")),
    by_extension: HashMap::from_iter(
        [
            (
                "md",
                include_str!("../assets/material-icon-theme/markdown.svg"),
            ),
            ("pdf", include_str!("../assets/material-icon-theme/pdf.svg")),
            (
                "doc",
                include_str!("../assets/material-icon-theme/word.svg"),
            ),
            (
                "docx",
                include_str!("../assets/material-icon-theme/word.svg"),
            ),
            (
                "xls",
                include_str!("../assets/material-icon-theme/table.svg"),
            ),
            (
                "xlsx",
                include_str!("../assets/material-icon-theme/table.svg"),
            ),
            (
                "ppt",
                include_str!("../assets/material-icon-theme/powerpoint.svg"),
            ),
            (
                "pptx",
                include_str!("../assets/material-icon-theme/powerpoint.svg"),
            ),
            (
                "json",
                include_str!("../assets/material-icon-theme/json.svg"),
            ),
            (
                "js",
                include_str!("../assets/material-icon-theme/javascript.svg"),
            ),
            (
                "ts",
                include_str!("../assets/material-icon-theme/typescript.svg"),
            ),
            (
                "py",
                include_str!("../assets/material-icon-theme/python.svg"),
            ),
            ("rs", include_str!("../assets/material-icon-theme/rust.svg")),
            ("css", include_str!("../assets/material-icon-theme/css.svg")),
            (
                "html",
                include_str!("../assets/material-icon-theme/html.svg"),
            ),
            ("zip", include_str!("../assets/material-icon-theme/zip.svg")),
            (
                "png",
                include_str!("../assets/material-icon-theme/image.svg"),
            ),
            (
                "mp4",
                include_str!("../assets/material-icon-theme/video.svg"),
            ),
            (
                "mp3",
                include_str!("../assets/material-icon-theme/audio.svg"),
            ),
            ("svg", include_str!("../assets/material-icon-theme/svg.svg")),
            (
                "sh",
                include_str!("../assets/material-icon-theme/console.svg"),
            ),
            (
                "lock",
                include_str!("../assets/material-icon-theme/lock.svg"),
            ),
            (
                "toml",
                include_str!("../assets/material-icon-theme/toml.svg"),
            ),
            (
                "yaml",
                include_str!("../assets/material-icon-theme/yaml.svg"),
            ),
            (
                "yml",
                include_str!("../assets/material-icon-theme/yaml.svg"),
            ),
            ("xml", include_str!("../assets/material-icon-theme/xml.svg")),
            ("nix", include_str!("../assets/material-icon-theme/nix.svg")),
            ("go", include_str!("../assets/material-icon-theme/go.svg")),
            (
                "java",
                include_str!("../assets/material-icon-theme/java.svg"),
            ),
            (
                "txt",
                include_str!("../assets/material-icon-theme/document.svg"),
            ),
            ("7z", include_str!("../assets/material-icon-theme/zip.svg")),
            ("rar", include_str!("../assets/material-icon-theme/zip.svg")),
        ]
        .map(|(extension, svg)| (extension, svg_glyph(svg))),
    ),
});

struct MatIcons {
    folder: icondata_core::IconData,
    folder_open: icondata_core::IconData,
    file: icondata_core::IconData,
    by_extension: HashMap<&'static str, icondata_core::IconData>,
}

/// [`iced_fabi::RowIcons`] over [`MATERIAL_ICONS`], looking files up by extension; the glyphs keep their own colours.
pub(crate) struct MaterialIconTheme;

impl iced_fabi::RowIcons for MaterialIconTheme {
    fn folder(&self, _node: &Node, open: bool) -> icondata::Icon {
        let icons = &*MATERIAL_ICONS;
        if open {
            &icons.folder_open
        } else {
            &icons.folder
        }
    }

    fn file(&self, node: &Node) -> icondata::Icon {
        let icons = &*MATERIAL_ICONS;
        icons
            .by_extension
            .get(extension(&node.name).unwrap_or(""))
            .unwrap_or(&icons.file)
    }

    fn tinted(&self) -> bool {
        false
    }
}
