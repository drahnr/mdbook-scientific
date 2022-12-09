pub mod error;
mod fragments;
mod preprocess;

use crate::error::Error;
use fs_err as fs;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use mdbook::book::{Book, BookItem, Chapter};
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use nom_bibtex::*;

use preprocess::{replace_blocks, replace_inline_blocks};

pub struct Scientific;

impl Scientific {
    pub fn new() -> Scientific {
        Scientific
    }
}

impl Preprocessor for Scientific {
    fn name(&self) -> &str {
        "scientific"
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        dbg!(renderer) != "not-supported"
            || !renderer.ends_with("latex")
            || !renderer.ends_with("tectonic")
    }

    fn run(&self, ctx: &PreprocessorContext, book: Book) -> Result<Book, mdbook::errors::Error> {
        self.run_inner(ctx, book)
            .map_err(mdbook::errors::Error::new)
    }
}

impl Scientific {
    fn run_inner(&self, ctx: &PreprocessorContext, mut book: Book) -> crate::error::Result<Book> {
        if let Some(cfg) = ctx.config.get_preprocessor(self.name()) {
            let fragment_path = cfg
                .get("fragment_path")
                .map(|x| x.as_str().expect("Fragment path is valid UTF8. qed"))
                .unwrap_or("fragments/");

            let fragment_path = Path::new(fragment_path);

            fs::create_dir_all(fragment_path)?;

            let fragment_path = fs::canonicalize(fragment_path)?;

            // track which fragments we use to copy them into the assets folder
            let mut used_fragments = Vec::new();
            // track which references are created
            let mut references = HashMap::new();
            // if there occurs an error skip everything and return the error
            let mut error = Ok::<_, Error>(());

            // load all references in the bibliography and export to html
            if let (Some(bib), Some(bib2xhtml)) = (cfg.get("bibliography"), cfg.get("bib2xhtml")) {
                let bib = bib.as_str().unwrap();
                let bib2xhtml = bib2xhtml.as_str().expect("bib string is valid UTF8. qed");

                if !Path::new(bib).exists() {
                    return Err(Error::BibliographyMissing(bib.to_owned()));
                }

                // read entries in bibtex file
                let bibtex = fs::read_to_string(bib)?;
                let bibtex = Bibtex::parse(&bibtex)?;
                for (i, entry) in bibtex.bibliographies().into_iter().enumerate() {
                    references.insert(entry.citation_key().to_string(), format!("[{}]", i + 1));
                }

                // create bibliography
                let content = fragments::bib_to_html(&bib, &bib2xhtml)?;

                // add final chapter for bibliography
                let bib_chapter = Chapter::new(
                    "Bibliography",
                    format!("# Bibliography\n{}", content),
                    PathBuf::from("bibliography.md"),
                    Vec::new(),
                );
                book.push_item(bib_chapter);
            }

            // assets path
            let asset_path = cfg
                .get("assets")
                .map(|x| x.as_str().expect("Assumes valid UTF8 for assets. qed"))
                .unwrap_or("src/");
            let asset_path = ctx.root.join(asset_path);

            // process blocks like `$$ .. $$`
            book.for_each_mut(|item| {
                if let Err(_) = error {
                    return;
                }

                if let BookItem::Chapter(ref mut ch) = item {
                    let head_number = ch
                        .number
                        .as_ref()
                        .map(|x| x.to_string())
                        .unwrap_or(String::new());

                    match replace_blocks(
                        &fragment_path,
                        &asset_path,
                        &ch.content,
                        &head_number,
                        &mut used_fragments,
                        &mut references,
                    ) {
                        Ok(x) => ch.content = x,
                        Err(err) => error = Err(Error::from(err)),
                    }
                }
            });

            // process inline blocks like `$ .. $`
            book.for_each_mut(|item| {
                if error.is_err() {
                    return;
                }

                if let BookItem::Chapter(ref mut ch) = item {
                    let _head_number = ch
                        .number
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default();

                    match replace_inline_blocks(
                        &fragment_path,
                        &ch.content,
                        &references,
                        &mut used_fragments,
                    ) {
                        Ok(x) => ch.content = x,
                        Err(err) => error = Err(Error::from(err)),
                    }
                }
            });

            error?;

            // the output path is `src/assets`, which get copied to the output directory
            let dest = ctx.root.join("src").join("storage").join("assets");
            if !dest.exists() {
                fs::create_dir_all(&dest)?;
            }

            // copy all fragments
            for fragment in used_fragments {
                fs::copy(fragment_path.join(&fragment), dest.join(&fragment))?;
            }

            Ok(book)
        } else {
            Err(Error::KeySectionNotFound)
        }
    }
}
