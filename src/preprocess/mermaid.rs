use super::*;

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
    let mut stdin = child.stdin.take().expect("Has stdin. qed");
    let j = std::thread::spawn(move || {
        stdin.write(code.as_bytes())?;
        Ok::<_, crate::errors::Error>(())
    });
    // let mut stdout = child.stdout.expect("Has stdout. qed");
    // let mut buf = String::with_capacity(8192);
    // stdout.read_to_string(&mut buf)?;

    j.join().unwrap()?;

    Ok(dest)
}

/// Replaces the content of the cmark file where codeblocks tagged with `mermaid`
/// so for
///
pub fn replace_mermaid_charts(
    source: &str,
    chapterno: String,
    dest: impl AsRef<Path>,
    renderer: SupportedRenderer,
    used_fragments: &mut Vec<PathBuf>,
) -> Result<String> {
    match renderer {
        // html can just fine deal with it
        SupportedRenderer::Html => return Ok(source.to_owned()),
        _ => {
            // eprintln!("Stripping `mermaid` fencing of code block, not supported yet")
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

    let mut events = vec![];
    let mut state = State::default();
    for (event, offset) in Parser::new_ext(&source, Options::all()).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(ref kind)) => match kind {
                CodeBlockKind::Fenced(s) if s.as_ref() == "mermaid" => {
                    state.counter += 1;
                    state.is_mermaid_block = true;
                    continue;
                }
                _ => {}
            },
            Event::End(Tag::CodeBlock(ref kind)) => match kind {
                CodeBlockKind::Fenced(s) if s.as_ref() == "mermaid" => {
                    state.is_mermaid_block = false;
                    continue;
                }
                _ => {}
            },

            Event::Text(ref code) | Event::Code(ref code) => {
                if state.is_mermaid_block {
                    let svg_path = create_svg_from_mermaid(
                        code.as_ref(),
                        dest,
                        chapterno.as_str(),
                        state.counter,
                    )?;
                    used_fragments.push(svg_path.clone());

                    let desc: CowStr =
                        format!("Chapter {}, Graphic {}", chapterno.as_str(), state.counter).into();
                    let title = desc.clone();
                    let inject = Tag::Image(
                        LinkType::Inline,
                        svg_path.display().to_string().into(),
                        title,
                    );

                    events.push(Event::Start(inject.clone()));
                    events.push(Event::Text(desc));
                    events.push(Event::End(inject));
                    continue;
                }
            }
            _ => {}
        }
        events.push(event);
    }

    pulldown_cmark_to_cmark::cmark(dbg!(events).into_iter(), &mut buf)
        .map_err(Error::CommonMarkGlue)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use pulldown_cmark::{Event, Options, Parser, Tag};
    use std::env::temp_dir;

    use super::*;

    #[test]
    fn gen_mermaid_svg_and_replace() {
        let dest = temp_dir().join(format!("mdboff"));
        fs::create_dir_all(&dest).unwrap();
        let adjusted = replace_mermaid_charts(
            r#"
```mermaid
graph
    A-->B
```
"#,
            "1.2.3".into(),
            dest,
            SupportedRenderer::Markdown,
            &mut Vec::new(),
        )
        .unwrap();

        let mut iter = Parser::new_ext(&adjusted, Options::all()).into_iter();

        let _ = iter.next();
        assert_matches!(dbg!(iter.next()), Some(Event::Start(Tag::Image(_, _, _))));
        assert_matches!(iter.next(), Some(Event::Text(s)) => {
            assert!(s.contains("1.2.3"));
        });
        assert_matches!(iter.next(), Some(Event::End(Tag::Image(_, _, _))));
    }
}
