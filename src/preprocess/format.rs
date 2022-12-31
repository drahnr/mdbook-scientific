use super::*;

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
            format!(r#"\[{}\]"#, replacement.inner_str_or_intermediate())
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
            format!(r#"\[{}\]"#, replacement.inner_str_or_intermediate())
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
            format!(r#"\[{}\]"#, replacement.inner_str_or_intermediate())
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
            format!(r#"${}$"#, replacement.inner_str_or_intermediate())
        }
    }
}
