use gpui::prelude::FluentBuilder;
use gpui::{div, px, AnyElement, App, FontStyle, FontWeight, InteractiveElement, IntoElement, ParentElement, Styled, Window};
use gpui_component::{scroll::ScrollableElement as _, Theme};
use pulldown_cmark::{Alignment, Event, Options, Parser, Tag, TagEnd};

#[derive(Clone, Copy, Debug)]
pub struct MarkdownRenderConfig {
    pub enable_obsidian_tags: bool,
}

impl Default for MarkdownRenderConfig {
    fn default() -> Self {
        Self {
            enable_obsidian_tags: false,
        }
    }
}

#[derive(Clone, Copy, Default, Debug)]
struct InlineStyle {
    bold: bool,
    italic: bool,
    strikethrough: bool,
}

#[derive(Debug)]
enum MdNode {
    Paragraph(Vec<MdNode>),
    Heading(u8, Vec<MdNode>),
    BlockQuote(Vec<MdNode>),
    List(bool, Vec<MdNode>),
    Item(bool, bool, Vec<MdNode>), // (is_task_list, checked, children)
    CodeInline(String, InlineStyle),
    Text(String, InlineStyle),
    Link(String, Vec<MdNode>),
    CodeBlock(String, String),
    Table(Vec<Alignment>, Vec<MdNode>),
    TableRow(bool, Vec<Alignment>, Vec<MdNode>),
    TableCell(Alignment, Vec<MdNode>),
    Rule,
    Html(String),
}

fn sanitize_markdown(content: &str) -> String {
    let mut s = content.replace('\r', "");
    s = s.replace("$\\rightarrow$", "→");
    s = s.replace("$\\Rightarrow$", "⇒");
    s = s.replace("\\rightarrow", "→");
    s = s.replace("\\Rightarrow", "⇒");
    s = s.replace("$\\\n", "$\\");
    s
}

fn parse_markdown(content: &str) -> Vec<MdNode> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(content, options);

    let mut stack: Vec<MdNode> = vec![MdNode::Paragraph(vec![])];
    let mut strong_depth = 0usize;
    let mut emphasis_depth = 0usize;
    let mut strikethrough_depth = 0usize;
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_content = String::new();
    let mut current_table_alignments: Vec<Alignment> = vec![];
    let mut current_cell_index = 0usize;

    for event in parser {
        let style = InlineStyle {
            bold: strong_depth > 0,
            italic: emphasis_depth > 0,
            strikethrough: strikethrough_depth > 0,
        };

        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => stack.push(MdNode::Paragraph(vec![])),
                Tag::Heading { level, .. } => stack.push(MdNode::Heading(level as u8, vec![])),
                Tag::BlockQuote(_) => stack.push(MdNode::BlockQuote(vec![])),
                Tag::List(start) => stack.push(MdNode::List(start.is_some(), vec![])),
                Tag::Item => stack.push(MdNode::Item(false, false, vec![])),
                Tag::Table(alignments) => {
                    current_table_alignments = alignments.clone();
                    stack.push(MdNode::Table(alignments, vec![]));
                }
                Tag::TableHead => {
                    current_cell_index = 0;
                    stack.push(MdNode::TableRow(true, current_table_alignments.clone(), vec![]));
                }
                Tag::TableRow => {
                    current_cell_index = 0;
                    stack.push(MdNode::TableRow(false, current_table_alignments.clone(), vec![]));
                }
                Tag::TableCell => {
                    let alignment = current_table_alignments.get(current_cell_index).copied().unwrap_or(Alignment::None);
                    stack.push(MdNode::TableCell(alignment, vec![]));
                    current_cell_index += 1;
                }
                Tag::Strong => strong_depth += 1,
                Tag::Emphasis => emphasis_depth += 1,
                Tag::Strikethrough => strikethrough_depth += 1,
                Tag::Link { dest_url, .. } => stack.push(MdNode::Link(dest_url.to_string(), vec![])),
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    code_content.clear();
                    code_lang = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                        pulldown_cmark::CodeBlockKind::Indented => String::new(),
                    };
                }
                _ => {}
            },
            Event::TaskListMarker(checked) => {
                if let Some(MdNode::Item(is_task_list, is_checked, _)) = stack.last_mut() {
                    *is_task_list = true;
                    *is_checked = checked;
                }
            }
            Event::Html(html) => {
                if let Some(parent) = stack.last_mut() {
                    match parent {
                        MdNode::Paragraph(children)
                        | MdNode::Heading(_, children)
                        | MdNode::BlockQuote(children)
                        | MdNode::List(_, children)
                        | MdNode::Item(_, _, children)
                        | MdNode::Table(_, children)
                        | MdNode::TableRow(_, _, children)
                        | MdNode::TableCell(_, children)
                        | MdNode::Link(_, children) => children.push(MdNode::Html(html.to_string())),
                        _ => {}
                    }
                }
            }
            Event::End(tag_end) => {
                let node = match tag_end {
                    TagEnd::Paragraph
                    | TagEnd::Heading(_)
                    | TagEnd::BlockQuote(_)
                    | TagEnd::List(_)
                    | TagEnd::Item
                    | TagEnd::Table
                    | TagEnd::TableRow
                    | TagEnd::TableCell
                    | TagEnd::Link => stack.pop(),
                    TagEnd::TableHead => stack.pop(),
                    TagEnd::Strong => {
                        strong_depth = strong_depth.saturating_sub(1);
                        None
                    }
                    TagEnd::Emphasis => {
                        emphasis_depth = emphasis_depth.saturating_sub(1);
                        None
                    }
                    TagEnd::Strikethrough => {
                        strikethrough_depth = strikethrough_depth.saturating_sub(1);
                        None
                    }
                    TagEnd::CodeBlock => {
                        in_code_block = false;
                        Some(MdNode::CodeBlock(code_lang.clone(), code_content.clone()))
                    }
                    _ => None,
                };

                if let Some(n) = node {
                    if let Some(parent) = stack.last_mut() {
                        match parent {
                            MdNode::Paragraph(children)
                            | MdNode::Heading(_, children)
                            | MdNode::BlockQuote(children)
                            | MdNode::List(_, children)
                            | MdNode::Item(_, _, children)
                            | MdNode::Table(_, children)
                            | MdNode::TableRow(_, _, children)
                            | MdNode::TableCell(_, children)
                            | MdNode::Link(_, children) => children.push(n),
                            _ => {}
                        }
                    }
                }
            }
            Event::Text(text) => {
                if in_code_block {
                    code_content.push_str(&text);
                } else if let Some(parent) = stack.last_mut() {
                    match parent {
                        MdNode::Paragraph(children)
                        | MdNode::Heading(_, children)
                        | MdNode::BlockQuote(children)
                        | MdNode::List(_, children)
                        | MdNode::Item(_, _, children)
                        | MdNode::Table(_, children)
                        | MdNode::TableRow(_, _, children)
                        | MdNode::TableCell(_, children)
                        | MdNode::Link(_, children) => {
                            children.push(MdNode::Text(text.to_string(), style));
                        }
                        _ => {}
                    }
                }
            }
            Event::Code(code) => {
                if let Some(parent) = stack.last_mut() {
                    match parent {
                        MdNode::Paragraph(children)
                        | MdNode::Heading(_, children)
                        | MdNode::BlockQuote(children)
                        | MdNode::List(_, children)
                        | MdNode::Item(_, _, children)
                        | MdNode::Table(_, children)
                        | MdNode::TableRow(_, _, children)
                        | MdNode::TableCell(_, children)
                        | MdNode::Link(_, children) => {
                            children.push(MdNode::CodeInline(code.to_string(), style));
                        }
                        _ => {}
                    }
                }
            }
            Event::SoftBreak => {
                if in_code_block {
                    code_content.push('\n');
                } else if let Some(parent) = stack.last_mut() {
                    match parent {
                        MdNode::Paragraph(children)
                        | MdNode::Heading(_, children)
                        | MdNode::BlockQuote(children)
                        | MdNode::List(_, children)
                        | MdNode::Item(_, _, children)
                        | MdNode::Table(_, children)
                        | MdNode::TableRow(_, _, children)
                        | MdNode::TableCell(_, children)
                        | MdNode::Link(_, children) => children.push(MdNode::Text(" ".to_string(), style)),
                        _ => {}
                    }
                }
            }
            Event::HardBreak => {
                if in_code_block {
                    code_content.push('\n');
                } else if let Some(parent) = stack.last_mut() {
                    match parent {
                        MdNode::Paragraph(children)
                        | MdNode::Heading(_, children)
                        | MdNode::BlockQuote(children)
                        | MdNode::List(_, children)
                        | MdNode::Item(_, _, children)
                        | MdNode::Table(_, children)
                        | MdNode::TableRow(_, _, children)
                        | MdNode::TableCell(_, children)
                        | MdNode::Link(_, children) => children.push(MdNode::Text("\n".to_string(), style)),
                        _ => {}
                    }
                }
            }
            Event::Rule => {
                if let Some(parent) = stack.last_mut() {
                    match parent {
                        MdNode::Paragraph(children)
                        | MdNode::Heading(_, children)
                        | MdNode::BlockQuote(children)
                        | MdNode::List(_, children)
                        | MdNode::Item(_, _, children)
                        | MdNode::Table(_, children)
                        | MdNode::TableRow(_, _, children)
                        | MdNode::TableCell(_, children)
                        | MdNode::Link(_, children) => children.push(MdNode::Rule),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    match stack.pop() {
        Some(MdNode::Paragraph(children)) => children,
        _ => vec![],
    }
}

fn render_inline(children: &[MdNode], theme: &Theme, cx: &mut Window, config: MarkdownRenderConfig) -> gpui::Div {
    let mut container = gpui::div().flex().flex_wrap().gap_x(px(4.)).w_full();
    for child in children {
        match child {
            MdNode::Text(text, style) => {
                if text == "\n" {
                    container = container.child(gpui::div().w_full().h(px(0.)));
                    continue;
                }

                for (line_ix, line) in text.split('\n').enumerate() {
                    if line_ix > 0 {
                        container = container.child(gpui::div().w_full().h(px(0.)));
                    }

                    for word in line.split_whitespace() {
                        let mut el = gpui::div().child(word.to_string());
                        if style.bold {
                            el = el.font_weight(FontWeight::BOLD);
                        }
                        if style.italic {
                            el.text_style()
                                .get_or_insert_with(Default::default)
                                .font_style = Some(FontStyle::Italic);
                        }
                        if style.strikethrough {
                            el.text_style()
                                .get_or_insert_with(Default::default)
                                .strikethrough = Some(gpui::StrikethroughStyle {
                                    thickness: px(1.0),
                                    color: Some(theme.foreground),
                                });
                        }

                        if config.enable_obsidian_tags {
                            if let Some(tag) = word.strip_prefix('#') {
                                if !tag.is_empty() {
                                    el = el
                                        .px(px(6.))
                                        .py(px(2.))
                                        .rounded_md()
                                        .bg(theme.secondary)
                                        .text_color(theme.accent);
                                }
                            }
                        }

                        container = container.child(el);
                    }
                }
            }
            MdNode::CodeInline(code, style) => {
                let mut el = gpui::div()
                    .bg(theme.secondary)
                    .px(px(4.))
                    .rounded_sm()
                    .font_family(theme.mono_font_family.clone())
                    .text_size(theme.mono_font_size)
                    .child(code.clone());
                if style.bold {
                    el = el.font_weight(FontWeight::BOLD);
                }
                if style.italic {
                    el.text_style()
                        .get_or_insert_with(Default::default)
                        .font_style = Some(FontStyle::Italic);
                }
                if style.strikethrough {
                    el.text_style()
                        .get_or_insert_with(Default::default)
                        .strikethrough = Some(gpui::StrikethroughStyle {
                            thickness: px(1.0),
                            color: Some(theme.foreground),
                        });
                }
                container = container.child(el);
            }
            MdNode::Link(url, link_children) => {
                let _url = url.clone();
                let label = render_inline(link_children, theme, cx, config)
                    .text_color(theme.accent)
                    .cursor_pointer();
                container = container.child(
                    gpui::div()
                        .child(label)
                        .on_mouse_down(gpui::MouseButton::Left, {
                            let url = url.clone();
                            move |_, _window: &mut Window, cx: &mut App| {
                                cx.open_url(&url);
                            }
                        })
                );
            }
            MdNode::Html(html) => {
                container = container.child(
                    gpui::div()
                        .child(html.clone())
                        .text_color(theme.muted_foreground)
                );
            }
            _ => container = container.child(render_block(child, theme, cx, config)),
        }
    }
    container
}

fn node_text_len(node: &MdNode) -> usize {
    match node {
        MdNode::Text(t, _) => t.trim().len(),
        MdNode::CodeInline(t, _) => t.trim().len(),
        MdNode::Link(_, children)
        | MdNode::Paragraph(children)
        | MdNode::Heading(_, children)
        | MdNode::BlockQuote(children)
        | MdNode::List(_, children)
        | MdNode::Item(_, _, children)
        | MdNode::Table(_, children)
        | MdNode::TableRow(_, _, children)
        | MdNode::TableCell(_, children) => children.iter().map(node_text_len).sum(),
        MdNode::CodeBlock(_, code) => code.trim().len(),
        MdNode::Rule => 0,
        MdNode::Html(t) => t.trim().len(),
    }
}

fn render_table(table_alignments: &[Alignment], rows: &[MdNode], theme: &Theme, cx: &mut Window, config: MarkdownRenderConfig) -> AnyElement {
    let mut col_count = table_alignments.len();
    for row in rows {
        if let MdNode::TableRow(_, _, cells) = row {
            col_count = col_count.max(cells.len());
        }
    }

    let mut col_lens = vec![0usize; col_count];
    for row in rows {
        if let MdNode::TableRow(_, _, cells) = row {
            for (i, cell) in cells.iter().enumerate() {
                let len = match cell {
                    MdNode::TableCell(_, children) => children.iter().map(node_text_len).sum(),
                    _ => node_text_len(cell),
                };
                if i < col_lens.len() {
                    col_lens[i] = col_lens[i].max(len);
                }
            }
        }
    }

    let col_widths: Vec<f32> = col_lens
        .into_iter()
        .map(|len| {
            let len = len.clamp(6, 40) as f32;
            (len * 7.0 + 48.0).clamp(160.0, 420.0)
        })
        .collect();

    let mut table = gpui::div()
        .flex_col()
        .border_1()
        .border_color(theme.border)
        .rounded_md();

    for row in rows {
        if let MdNode::TableRow(is_header, alignments, cells) = row {
            let mut row_el = gpui::div()
                .flex()
                .flex_row()
                .w_full()
                .border_b_1()
                .border_color(theme.border);
            if *is_header {
                row_el = row_el.bg(theme.secondary);
            }

            for col_ix in 0..col_count {
                let width = col_widths.get(col_ix).copied().unwrap_or(200.0);
                let align = alignments
                    .get(col_ix)
                    .copied()
                    .or_else(|| table_alignments.get(col_ix).copied())
                    .unwrap_or(Alignment::None);
                let is_last_col = col_ix + 1 == col_count;

                let mut cell_el = gpui::div()
                    .w(px(width))
                    .flex_none()
                    .p_2()
                    .when(!is_last_col, |this| {
                        this.border_r_1().border_color(theme.border)
                    });

                match align {
                    Alignment::Center => cell_el = cell_el.flex().justify_center(),
                    Alignment::Right => cell_el = cell_el.flex().justify_end(),
                    _ => {}
                }

                if let Some(cell) = cells.get(col_ix) {
                    match cell {
                        MdNode::TableCell(_, children) => {
                            cell_el = cell_el.child(render_inline(children, theme, cx, config).w_full());
                        }
                        _ => cell_el = cell_el.child(render_block(cell, theme, cx, config)),
                    }
                }

                row_el = row_el.child(cell_el);
            }

            table = table.child(row_el);
        } else {
            table = table.child(render_block(row, theme, cx, config));
        }
    }

    gpui::div()
        .w_full()
        .overflow_x_scrollbar()
        .child(table)
        .into_any_element()
}

fn render_block(node: &MdNode, theme: &Theme, cx: &mut Window, config: MarkdownRenderConfig) -> AnyElement {
    match node {
        MdNode::Paragraph(children) => render_inline(children, theme, cx, config).mb(px(8.)).into_any_element(),
        MdNode::Heading(level, children) => {
            let size = match level {
                1 => px(24.),
                2 => px(20.),
                3 => px(18.),
                _ => px(16.),
            };
            gpui::div()
                .w_full()
                .text_size(size)
                .font_weight(FontWeight::BOLD)
                .mt(px(8.))
                .mb(px(8.))
                .child(render_inline(children, theme, cx, config))
                .into_any_element()
        }
        MdNode::BlockQuote(children) => gpui::div()
            .w_full()
            .pl(px(12.))
            .border_l_4()
            .border_color(theme.border)
            .text_color(theme.muted_foreground)
            .mb(px(8.))
            .child(render_inline(children, theme, cx, config))
            .into_any_element(),
        MdNode::List(ordered, children) => {
            let mut container = gpui::div().w_full().flex_col().mb(px(8.)).pl(px(16.));
            for (i, child) in children.iter().enumerate() {
                let prefix = if *ordered {
                    format!("{}. ", i + 1)
                } else {
                    "• ".to_string()
                };
                container = container.child(
                    gpui::div()
                        .w_full()
                        .flex()
                        .gap_2()
                        .items_start()
                        .child(gpui::div().child(prefix))
                        .child(render_block(child, theme, cx, config)),
                );
            }
            container.into_any_element()
        }
        MdNode::Item(is_task, is_checked, children) => {
            let mut item_container = gpui::div().w_full().flex().gap_2();
            if *is_task {
                let checkbox = if *is_checked { "☑" } else { "☐" };
                item_container = item_container.child(
                    gpui::div()
                        .font_family(theme.mono_font_family.clone())
                        .child(checkbox)
                );
            }
            item_container.child(render_inline(children, theme, cx, config)).into_any_element()
        }
        MdNode::CodeBlock(lang, code) => gpui::div()
            .w_full()
            .mb(px(8.))
            .overflow_x_scrollbar()
            .child(
                gpui::div()
                    .w_full()
                    .bg(theme.secondary)
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .font_family(theme.mono_font_family.clone())
                    .text_size(theme.mono_font_size)
                    .child({
                        let mut col = gpui_component::v_flex().w_full().gap_2();
                        if !lang.trim().is_empty() {
                            col = col.child(div().text_xs().text_color(theme.muted_foreground).child(lang.clone()));
                        }
                        col.child(div().child(code.clone()))
                    }),
            )
            .into_any_element(),
        MdNode::Table(alignments, rows) => render_table(alignments, rows, theme, cx, config),
        MdNode::TableRow(_, _, _) | MdNode::TableCell(_, _) => div().into_any_element(),
        MdNode::Rule => gpui::div()
            .w_full()
            .h(px(1.))
            .bg(theme.border)
            .my(px(8.))
            .into_any_element(),
        MdNode::Html(html) => gpui::div()
            .text_color(theme.muted_foreground)
            .child(html.clone())
            .into_any_element(),
        MdNode::Text(_, _) | MdNode::CodeInline(_, _) | MdNode::Link(_, _) => {
            render_inline(std::slice::from_ref(node), theme, cx, config).into_any_element()
        }
    }
}

pub fn render_markdown_message(content: &str, theme: &Theme, cx: &mut Window) -> impl IntoElement {
    render_markdown_message_with_config(content, theme, cx, MarkdownRenderConfig::default())
}

pub fn render_markdown_message_with_config(
    content: &str,
    theme: &Theme,
    cx: &mut Window,
    config: MarkdownRenderConfig,
) -> impl IntoElement {
    let content = sanitize_markdown(content);
    let ast = parse_markdown(&content);
    let mut body = gpui_component::v_flex()
        .w_full()
        .min_w(px(0.))
        .text_size(px(14.))
        .text_color(theme.foreground)
        .line_height(gpui::relative(1.5));
    for node in ast {
        body = body.child(render_block(&node, theme, cx, config));
    }
    body
}
