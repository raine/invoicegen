use std::collections::HashMap;
use std::sync::OnceLock;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Dict, Value};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

static INTER_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
static INTER_BOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-Bold.ttf");
static INTER_ITALIC: &[u8] = include_bytes!("../../assets/fonts/Inter-Italic.ttf");
static INTER_BOLD_ITALIC: &[u8] = include_bytes!("../../assets/fonts/Inter-BoldItalic.ttf");

fn load_fonts() -> (FontBook, Vec<Font>) {
    let mut book = FontBook::new();
    let mut fonts = Vec::new();
    for blob in [INTER_REGULAR, INTER_BOLD, INTER_ITALIC, INTER_BOLD_ITALIC] {
        for font in Font::iter(Bytes::new(blob)) {
            book.push(font.info().clone());
            fonts.push(font);
        }
    }
    (book, fonts)
}

fn fonts() -> &'static (FontBook, Vec<Font>) {
    static CACHE: OnceLock<(FontBook, Vec<Font>)> = OnceLock::new();
    CACHE.get_or_init(load_fonts)
}

pub struct InvoiceWorld {
    main_id: FileId,
    main_source: Source,
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    files: HashMap<FileId, Bytes>,
}

impl InvoiceWorld {
    pub fn new(
        template_src: String,
        invoice_json: String,
        logo_bytes: Option<Vec<u8>>,
        logo_virtual_name: Option<String>,
    ) -> Self {
        let main_id = FileId::new(None, VirtualPath::new("/invoice.typ"));
        let main_source = Source::new(main_id, template_src);

        let mut inputs = Dict::new();
        inputs.insert("invoice_json".into(), Value::Str(invoice_json.into()));

        let library = Library::builder().with_inputs(inputs).build();

        let (book, fonts) = fonts();

        let mut files: HashMap<FileId, Bytes> = HashMap::new();
        if let (Some(bytes), Some(name)) = (logo_bytes, logo_virtual_name) {
            let id = FileId::new(None, VirtualPath::new(&name));
            files.insert(id, Bytes::new(bytes));
        }

        Self {
            main_id,
            main_source,
            library: LazyHash::new(library),
            book: LazyHash::new(book.clone()),
            fonts: fonts.clone(),
            files,
        }
    }
}

impl World for InvoiceWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }
    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main_id {
            Ok(self.main_source.clone())
        } else {
            Err(FileError::NotFound(
                id.vpath().as_rooted_path().to_path_buf(),
            ))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files
            .get(&id)
            .cloned()
            .ok_or_else(|| FileError::NotFound(id.vpath().as_rooted_path().to_path_buf()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}
