use fs_err as fs;
use itertools::Itertools;
use sha2::Digest;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::errors::{Error, Result};
use crate::fragments;
use crate::types::*;

const BLOCK_DELIM: &str = "$$";
const INLINE_BLOCK_DELIM: &str = "$";

#[cfg(test)]
mod tests;

pub fn format_figure<'a>(
    replacement: &Replacement<'a>,
    refer: &str,
    head_num: &str,
    figures_counter: usize,
    title: &str,
    renderer: SupportedRenderer,
) -> String {
    use SupportedRenderer::*;
    match renderer {
        Html | Markdown => {
            format!(
                r#"<figure id="{refer}" class="figure">
                    <object data="assets/{file}" type="image/svg+xml"/></object>
                    <figcaption>Figure {head_num}{figures_counter} {title}</figcaption>
                </figure>"#,
                refer = refer,
                head_num = head_num,
                figures_counter = figures_counter,
                title = title,
                file = replacement.svg.display()
            )
        }
        Latex | Tectonic => {
            format!(r#"\[{}\]"#, replacement.intermediate())
        }
    }
}

pub fn format_equation_block<'a>(
    replacement: &Replacement<'a>,
    refer: &str,
    head_num: &str,
    equations_counter: usize,
    renderer: SupportedRenderer,
) -> String {
    use SupportedRenderer::*;
    match renderer {
        Html | Markdown => {
            format!(
                r#"<div id="{refer}" class="equation">
                    <div class="equation_inner">
                        <object data="assets/{file}" type="image/svg+xml"></object>
                    </div><span>({head_num}{equations_counter})</span>
                </div>"#,
                refer = refer,
                head_num = head_num,
                equations_counter = equations_counter,
                file = replacement.svg.display()
            )
        }
        Latex | Tectonic => {
            format!(r#"\[{}\]"#, replacement.intermediate())
        }
    }
}

pub fn format_equation<'a>(replacement: &Replacement<'a>, renderer: SupportedRenderer) -> String {
    use SupportedRenderer::*;
    match renderer {
        Html | Markdown => {
            format!(
                r#"<div class="equation"><div class="equation_inner"><object data="assets/{file}" type="image/svg+xml"></object></div></div>\n"#,
                file = replacement.svg.display()
            )
        }
        Latex | Tectonic => {
            format!(r#"\[{}\]"#, replacement.intermediate())
        }
    }
}

pub fn format_inline_equation<'a>(
    replacement: &Replacement<'a>,
    renderer: SupportedRenderer,
) -> String {
    use SupportedRenderer::*;
    match renderer {
        Html | Markdown => {
            format!(
                r#"<object class="equation_inline" data="assets/{file}" type="image/svg+xml"></object>"#,
                file = replacement.svg.display()
            )
        }
        Latex | Tectonic => {
            format!(r#"${}$"#, replacement.content.s)
        }
    }
}

fn create_svg_from_mermaid(
    code: &str,
    dest: impl AsRef<Path>,
    chapterno: &str,
    counter: usize,
) -> Result<PathBuf> {
    let mmdc = which::which("mmdc")?;
    let dest = dest.as_ref();

    let dest = dest.join(format!("mermaid_{}_{}.svg", chapterno, counter));

    let mut child = std::process::Command::new(mmdc)
        .arg("--outputFormat=svg")
        .arg(format!("--output={}", dest.display()))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    // FIXME make this simpler
    let code = code.to_owned();
    let mut stdin = child.stdin.take().unwrap();
    let j = std::thread::spawn(move || {
        stdin.write(code.as_bytes())?;
        Ok::<_, crate::errors::Error>(())
    });
    let mut stdout = child.stdout.unwrap();
    let mut buf = String::with_capacity(8192);
    stdout.read_to_string(&mut buf)?;

    j.join().unwrap()?;
    dbg!(buf);

    Ok(dest)
}

/// Currently there is no way to display mermaid
/// TODO FIXME
pub fn gen_mermaid_charts(
    source: &str,
    chapterno: String,
    dest: impl AsRef<Path>,
    renderer: SupportedRenderer,
) -> Result<String> {
    match renderer {
        // markdown and html can just fine deal with it
        SupportedRenderer::Html => return Ok(source.to_owned()),
        // SupportedRenderer::Markdown => return Ok(source.to_owned()),
        _ => {
            eprintln!("Stripping `mermaid` fencing of code block, not supported yet")
        }
    }

    let dest = dest.as_ref();

    use pulldown_cmark::*;
    use pulldown_cmark_to_cmark::cmark;

    let mut buf = String::with_capacity(source.len());

    #[derive(Debug, Default)]
    struct State {
        is_mermaid_block: bool,
        counter: usize,
    }

    let events = Parser::new_ext(&source, Options::all())
        .into_offset_iter()
        .scan(State::default(), |state, (mut event, _offset)| {
            match event {
                Event::Start(Tag::CodeBlock(ref mut kind)) => match kind {
                    CodeBlockKind::Fenced(s) if s.as_ref() == "mermaid" => {
                        *kind = CodeBlockKind::Fenced("text".into());
                        state.counter += 1;
                        state.is_mermaid_block = true;
                        return None;
                    }
                    _ => {}
                },
                Event::End(Tag::CodeBlock(ref mut kind)) => match kind {
                    CodeBlockKind::Fenced(s) if s.as_ref() == "mermaid" => {
                        *kind = CodeBlockKind::Fenced("text".into());
                        state.is_mermaid_block = false;
                        return None;
                    }
                    _ => {}
                },
                Event::Code(ref code) => {
                    if state.is_mermaid_block {
                        let svg_path = dbg!(create_svg_from_mermaid(
                            code.as_ref(),
                            dest,
                            chapterno.as_str(),
                            state.counter
                        )
                        .expect("mermaid graph issue"));
                        let inject = Tag::Image(
                            LinkType::Inline,
                            "url".into(),
                            format!("Chapter {} Graphic {}", chapterno.as_str(), state.counter)
                                .into(),
                        );
                    }
                }
                _ => {}
            }
            Some(event)
        });

    pulldown_cmark_to_cmark::cmark(events, &mut buf).map_err(Error::CommonMarkGlue)?;
    Ok(buf)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SplitTagPosition<'a> {
    /// Position in line + columns
    lico: LiCo,
    /// Offset in bytes from the beginning of the string
    byte_offset: usize,
    /// start or end
    which: Dollar<'a>,
}

fn dollar_split_tags_iter<'a>(source: &'a str) -> impl Iterator<Item = SplitTagPosition<'a>> {
    let mut is_code_block = false;
    let mut is_pre_block = false;
    let mut is_dollar_block = false;
    source
        .lines()
        .scan(0_usize, |state, line_content| {
            let previous_line_char_count = *state;
            let current_char_count = line_content.chars().count();
            *state = current_char_count;
            Some((previous_line_char_count, current_char_count, line_content))
            // provide the previous line length and the current
        })
        .enumerate()
        .scan(
            0,
            move |state, (lineno, (previous_char_cnt, current_char_cnt, line_content))| {
                // handle block content

                let byte_offset = *state;
                *state += current_char_cnt + 1;

                // the end of the previous line
                let _previous = LiCo {
                    lineno: lineno.saturating_sub(1),
                    column: previous_char_cnt,
                };
                let mut current = LiCo { lineno, column: 1 };

                // FIXME NOT OK, could also be further in
                if line_content.starts_with("<pre") {
                    is_pre_block = true;
                    return None;
                }

                if line_content.starts_with("</pre>") {
                    is_pre_block = false;
                    return None;
                }

                if is_pre_block {
                    return None;
                }

                // FIXME use a proper markdown/commonmark parser, it's unfixable this
                // way i.e pre start and end in one line or multiple..
                if line_content.starts_with("```") {
                    is_code_block = !is_code_block;
                }
                if is_code_block {
                    return None;
                }

                if line_content.starts_with("$$") {
                    is_dollar_block = !is_dollar_block;
                    return Some(
                        vec![SplitTagPosition {
                            which: if is_dollar_block {
                                Dollar::Start(&line_content[..("$$".len())])
                            } else {
                                Dollar::End(&line_content[..("$$".len())])
                            },
                            lico: current,
                            byte_offset,
                            // char_offset, // TODO
                        }]
                        .into_iter(),
                    );
                }

                let mut is_intra_inline_code = false;
                let mut is_between_dollar_content = false;

                // use to collect ranges
                let mut v = Vec::from_iter(line_content.char_indices().enumerate().filter_map(
                    |(il_char_offset, (il_byte_offset, c))| {
                        match c {
                            '$' if !is_intra_inline_code => {
                                is_between_dollar_content = !is_between_dollar_content;
                                current.column = il_char_offset;
                                let dollar = SplitTagPosition {
                                    which: if is_between_dollar_content {
                                        Dollar::Start(&line_content[il_byte_offset..][..1])
                                    } else {
                                        Dollar::End(&line_content[il_byte_offset..][..1])
                                    },
                                    lico: current,
                                    byte_offset: byte_offset + il_byte_offset,
                                };
                                return Some(dollar);
                            }
                            '`' => {
                                is_intra_inline_code = !is_intra_inline_code;
                            }
                            _ => {}
                        }
                        None
                    },
                ));

                if v.len() & 0x1 != 0 {
                    let last = v.last().unwrap();
                    eprintln!("Inserting $-sign at end of line #{lineno}!");
                    v.push(SplitTagPosition {
                        lico: LiCo {
                            lineno,
                            column: current_char_cnt + 1,
                        },
                        byte_offset: line_content.len(),
                        which: Dollar::End(""),
                    })
                }
                Some(v.into_iter())
            },
        )
        .flatten()
}

#[derive(Debug, Clone)]
enum Tagged<'a> {
    Replace(Content<'a>),
    Keep(Content<'a>),
}

impl<'a> Into<Content<'a>> for Tagged<'a> {
    fn into(self) -> Content<'a> {
        match self {
            Self::Replace(c) => c,
            Self::Keep(c) => c,
        }
    }
}

impl<'a> AsRef<Content<'a>> for Tagged<'a> {
    fn as_ref(&self) -> &Content<'a> {
        match self {
            Self::Replace(ref c) => c,
            Self::Keep(ref c) => c,
        }
    }
}

fn iter_over_dollar_encompassed_blocks<'a>(
    source: &'a str,
    iter: impl Iterator<Item = SplitTagPosition<'a>>,
) -> impl Iterator<Item = Tagged<'a>> {
    // make sure the first part is kept if it doesn't start with a dollar sign
    let mut iter = iter.peekable();
    let pre = match iter.peek() {
        Some(nxt) if dbg!(nxt.byte_offset) > 0 => {
            let byte_range = 0..(nxt.byte_offset);
            let s = &source[byte_range.clone()];
            Some(dbg!(Tagged::Keep(Content {
                // content without the $ delimiters FIXME
                s,
                start: LiCo {
                    lineno: 0,
                    column: 0,
                },
                end: nxt.lico,
                byte_range,
                delimiter: Dollar::Empty
            })))
        }
        _ => None,
    };
    let iter = iter.tuple_windows().enumerate().map(
        move |(
            idx,
            (
                start @ SplitTagPosition {
                    byte_offset: start_byte_offset,
                    which: start_which,
                    ..
                },
                end @ SplitTagPosition {
                    byte_offset: end_byte_offset,
                    which: end_which,
                    ..
                },
            ),
        )| {
            let replace = idx & 0x1 == 0;
            let byte_range = if replace {
                // replace must _include_ the `$`-signs
                start_byte_offset..(end_byte_offset + end_which.as_ref().len())
            } else {
                // first character might not exist, so this was injected and hence
                // would skip the first character
                let skip_dollar = if start_byte_offset == 0 {
                    0
                } else {
                    {
                        start_which.as_ref().len()
                    }
                };
                (start_byte_offset + skip_dollar)..end_byte_offset
            };

            // not within, so just return a string
            let content = Content {
                // content without the $ delimiters FIXME
                s: &source[byte_range.clone()],
                start: start.lico,
                end: end.lico,
                byte_range,
                delimiter: start.which,
            };

            if replace {
                Tagged::Replace(content)
            } else {
                Tagged::Keep(content)
            }
        },
    );
    pre.into_iter().chain(iter)
}

pub fn replace_blocks(
    fragment_path: impl AsRef<Path>,
    asset_path: impl AsRef<Path>,
    source: &str,
    head_num: &str,
    renderer: SupportedRenderer,
    used_fragments: &mut Vec<PathBuf>,
    references: &mut HashMap<String, String>,
) -> Result<String> {
    let fragment_path = fragment_path.as_ref();
    fs::create_dir_all(fragment_path)?;

    let iter = dollar_split_tags_iter(source);
    let s = iter_over_dollar_encompassed_blocks(source, iter)
        .map(|tagged| {
            let content = tagged.as_ref();
            // let mut dollarless_range = content.byte_range.clone();
            let regex = regex::Regex::new(r###"^\$+(.+)\$+"###).unwrap();
            let dollarless = regex.replace_all(content.as_ref(), "$1");
            let mut content = content.clone();
            // a bit bonkers FIXME XXX incoherent datastructure
            content.s = dollarless.as_ref();

            if !content.delimiter.is_block() {
                transform_block_as_needed(
                    &content,
                    fragment_path,
                    head_num,
                    references,
                    used_fragments,
                    renderer,
                )
            } else {
                transform_inline_as_needed(
                    &content,
                    fragment_path,
                    head_num,
                    references,
                    used_fragments,
                    renderer,
                )
            }
        })
        .collect::<Result<Vec<String>>>()?
        .into_iter()
        .join("\n");
    Ok(s)
}

fn transform_inline_as_needed<'a>(
    dollarless: &Content<'a>,
    fragment_path: impl AsRef<Path>,
    head_num: &str,
    references: &mut HashMap<String, String>,
    used_fragments: &mut Vec<PathBuf>,
    renderer: SupportedRenderer,
) -> Result<String> {
    let fragment_path = fragment_path.as_ref();
    let lineno = dollarless.start.lineno;
    let content = dollarless;

    let mut figures_counter = 0;
    let mut equations_counter = 0;

    if let Some(stripped) = dollarless.strip_prefix("ref:") {
        let mut add_object =
            move |replacement: &Replacement<'_>, refer: &str, title: Option<&str>| -> String {
                let file = replacement.svg.as_path();
                used_fragments.push(file.to_owned());

                if let Some(title) = title {
                    figures_counter += 1;
                    references.insert(
                        refer.to_string(),
                        format!("Figure {}{}", head_num, figures_counter),
                    );

                    format_figure(
                        replacement,
                        refer,
                        head_num,
                        figures_counter,
                        title,
                        renderer,
                    )
                } else if !refer.is_empty() {
                    equations_counter += 1;
                    references.insert(
                        refer.to_string(),
                        format!("{}{}", head_num, equations_counter),
                    );
                    format_equation_block(replacement, refer, head_num, equations_counter, renderer)
                } else {
                    format_equation(replacement, renderer)
                }
            };

        let elms = stripped.split(':').collect::<Vec<&str>>();
        match &elms[..] {
            ["latex", refer, title] => fragments::parse_latex(fragment_path, &content)
                .map(|ref file| add_object(file, refer, Some(title))),
            ["gnuplot", refer, title] => fragments::parse_gnuplot(fragment_path, &content)
                .map(|ref file| add_object(file, refer, Some(title))),
            ["gnuplotonly", refer, title] => fragments::parse_gnuplot_only(fragment_path, &content)
                .map(|ref file| add_object(file, refer, Some(title))),

            ["equation", refer] | ["equ", refer] => {
                fragments::generate_replacement_file_from_template(fragment_path, &content, 1.6)
                    .map(|ref file| add_object(file, refer, None))
            }

            ["equation"] | ["equ"] | _ => {
                fragments::generate_replacement_file_from_template(fragment_path, &content, 1.6)
                    .map(|ref file| add_object(file, "", None))
            }

            [kind, _] => Err(Error::UnknownReferenceKind {
                kind: kind.to_owned().to_owned(),
                lineno,
            }),
            _ => Err(Error::UnexpectedReferenceArgCount {
                count: elms.len(),
                lineno,
            }),
        }
    } else {
        fragments::generate_replacement_file_from_template(fragment_path, &dollarless, 1.3).map(
            |replacement| {
                let res = format_inline_equation(&replacement, renderer);
                used_fragments.push(replacement.svg);
                res
            },
        )
    }
}

/// `s` is the content withou
fn transform_block_as_needed<'a>(
    dollarless: &Content<'a>,
    fragment_path: impl AsRef<Path>,
    head_num: &str,
    references: &HashMap<String, String>,
    used_fragments: &mut Vec<PathBuf>,
    renderer: SupportedRenderer,
) -> Result<String> {
    let fragment_path = fragment_path.as_ref();
    let lineno = dollarless.start.lineno;
    if let Some(stripped) = dollarless.strip_prefix("ref:") {
        let elms = stripped.split(':').collect::<Vec<&str>>();
        match &elms[..] {
            ["fig", refere] => references
                .get::<str>(refere)
                .ok_or(Error::InvalidReference {
                    to: elms[1].to_owned(),
                    lineno,
                })
                .map(|x| format!(r#"<a class="fig_ref" href='#{}'>{}</a>"#, elms[1], x)),
            ["bib", refere] => references
                .get::<str>(refere)
                .ok_or(Error::InvalidReference {
                    to: elms[1].to_owned(),
                    lineno,
                })
                .map(|x| {
                    format!(
                        r#"<a class="bib_ref" href='bibliography.html#{}'>{}</a>"#,
                        elms[1], x
                    )
                }),
            ["equ", refere] => references
                .get::<str>(refere)
                .ok_or(Error::InvalidReference {
                    to: elms[1].to_owned(),
                    lineno,
                })
                .map(|x| format!(r#"<a class="equ_ref" href='#{}'>Eq. ({})</a>"#, elms[1], x)),

            [kind, _] => Err(Error::UnknownReferenceKind {
                kind: kind.to_owned().to_owned(),
                lineno,
            }),
            _ => Err(Error::UnexpectedReferenceArgCount {
                count: elms.len(),
                lineno,
            }),
        }
    } else {
        fragments::generate_replacement_file_from_template(fragment_path, &dollarless, 1.3).map(
            |replacement| {
                let res = format_inline_equation(&replacement, renderer);
                used_fragments.push(replacement.svg);
                res
            },
        )
    }
}
