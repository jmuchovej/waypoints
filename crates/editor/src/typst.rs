use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Mutex;

use typst::foundations::Bytes;
use typst::layout::Abs;
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt};
use typst_syntax::{FileId, Source, VirtualPath};

/// Minimal Typst World for rendering math expressions to SVG.
struct MathWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    source: Source,
    fonts: Vec<Font>,
}

impl MathWorld {
    fn new(expression: &str, display_mode: bool) -> Self {
        // Wrap expression in a typst document that renders math with minimal page
        let source_text = if display_mode {
            format!(
                "#set page(width: auto, height: auto, margin: 2pt)\n$ {} $",
                expression
            )
        } else {
            format!(
                "#set page(width: auto, height: auto, margin: 2pt)\n${}$",
                expression
            )
        };

        let file_id = FileId::new(None, VirtualPath::new("/main.typ"));
        let source = Source::new(file_id, source_text);

        // Load bundled fonts from typst-assets
        let mut book = FontBook::new();
        let mut fonts = Vec::new();
        for font_data in typst_assets::fonts() {
            let buffer = Bytes::new(font_data);
            for font in Font::iter(buffer) {
                book.push(font.info().clone());
                fonts.push(font);
            }
        }

        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            source,
            fonts,
        }
    }
}

impl typst::World for MathWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> typst::diag::FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            Err(typst::diag::FileError::NotFound(
                id.vpath().as_rootless_path().into(),
            ))
        }
    }

    fn file(&self, id: FileId) -> typst::diag::FileResult<Bytes> {
        Err(typst::diag::FileError::NotFound(
            id.vpath().as_rootless_path().into(),
        ))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<typst::foundations::Datetime> {
        None
    }
}

// Simple expression → SVG cache
static CACHE: std::sync::LazyLock<Mutex<HashMap<u64, String>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

fn hash_key(expression: &str, display_mode: bool) -> u64 {
    let mut h = DefaultHasher::new();
    expression.hash(&mut h);
    display_mode.hash(&mut h);
    h.finish()
}

/// Render a math expression to SVG using the Typst compiler.
pub fn render_math_to_svg(expression: &str, display_mode: bool) -> Result<String, String> {
    let key = hash_key(expression, display_mode);

    // Check cache first
    if let Ok(cache) = CACHE.lock() {
        if let Some(svg) = cache.get(&key) {
            return Ok(svg.clone());
        }
    }

    let world = MathWorld::new(expression, display_mode);

    let document = typst::compile::<typst::layout::PagedDocument>(&world)
        .output
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|e| e.message.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        })?;

    let svg = typst_svg::svg_merged(&document, Abs::zero());

    // Cache the result
    if let Ok(mut cache) = CACHE.lock() {
        cache.insert(key, svg.clone());
    }

    Ok(svg)
}
