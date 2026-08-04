use chrono::Utc;
use gpui::prelude::FluentBuilder;
use gpui::{
    div, fill, point, px, quad, App, AppContext, BorderStyle, Bounds, Context, Edges, Element,
    ElementId, GlobalElementId, HighlightStyle, InspectorElementId, InteractiveElement,
    IntoElement, LayoutId, ParentElement, Pixels, Point, SharedString, StatefulInteractiveElement,
    Styled, StyledText, TextRun, UnderlineStyle, Window,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::select::{Select, SelectState};
use gpui_component::tab::{Tab, TabBar};
use gpui_component::IndexPath;
use gpui_component::WindowExt;
use gpui_component::{h_flex, ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _};
use std::ops::Range;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::slash_commands::{
    build_slash_prompt, filter_slash_commands, parse_slash_invocation, slash_help_text,
    slash_query_from_input, SlashCommandId, SlashCommandParseError, SlashCommandTone,
};
use super::{PendingChatAction, TeamWorkspacePanel};
use crate::ui::components::markdown::render_markdown_message;

fn provider_kind(p: &crate::db::Provider) -> &str {
    crate::application::services::provider_factory::provider_kind(p)
}

fn build_provider_adapter(
    provider: &crate::db::Provider,
) -> Option<Arc<dyn crate::providers::BaseProviderAdapter>> {
    crate::application::services::provider_factory::create_adapter(provider)
}

fn format_session_label(s: &crate::core::models::session::SessionRecord) -> String {
    chrono::DateTime::parse_from_rfc3339(&s.created_at)
        .ok()
        .map(|d| d.with_timezone(&chrono::Local))
        .map(|d| d.format("%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "Session".to_string())
}

const AI_THINKING_LABEL: &str = "AI thinking";
const CHAT_COLLAPSE_LINE_LIMIT: usize = 80;
const CHAT_COLLAPSE_CHAR_LIMIT: usize = 4_000;
const CHAT_RENDER_CHAR_LIMIT: usize = 30_000;

fn agent_avatar_color(seed: &str) -> gpui::Hsla {
    let mut hash = 0u32;
    for byte in seed.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
    }
    let palette = [
        0xDBEAFEFF, 0xDCFCE7FF, 0xFEF3C7FF, 0xFCE7F3FF, 0xE0E7FFFF, 0xCCFBF1FF, 0xFAE8FFFF,
        0xFFE4E6FF, 0xEDE9FEFF, 0xECFCCBFF,
    ];
    gpui::Hsla::from(gpui::rgba(palette[(hash as usize) % palette.len()]))
}

fn chat_message_metadata(agent_name: &str, thought_duration_secs: Option<f64>) -> String {
    let mut metadata = serde_json::json!({ "agent_name": agent_name });
    if let Some(seconds) = thought_duration_secs {
        metadata["thought_duration_secs"] = serde_json::json!(seconds);
    }
    metadata.to_string()
}

fn format_thought_duration(seconds: f64) -> String {
    if seconds < 10.0 {
        format!("{:.1}s", seconds)
    } else {
        format!("{:.0}s", seconds)
    }
}

fn url_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut offset = 0;

    for part in text.split_whitespace() {
        let Some(relative_start) = text[offset..].find(part) else {
            continue;
        };
        let start = offset + relative_start;
        let mut end = start + part.len();

        while end > start {
            let Some(ch) = text[..end].chars().next_back() else {
                break;
            };
            if matches!(ch, ',' | '.' | ')' | ']' | '}' | '"' | '\'' | '>' | '<') {
                end -= ch.len_utf8();
            } else {
                break;
            }
        }

        let candidate = &text[start..end];
        if candidate.starts_with("http://") || candidate.starts_with("https://") {
            ranges.push(start..end);
        }

        offset = start + part.len();
    }

    ranges
}

struct LinkInlineOverlay {
    id: ElementId,
    text: SharedString,
    ranges: Vec<Range<usize>>,
    styled_text: StyledText,
}

impl LinkInlineOverlay {
    fn new(text: impl Into<SharedString>, ranges: Vec<Range<usize>>) -> Self {
        let text = text.into();
        Self {
            id: ElementId::Name("chat-link-inline-overlay".into()),
            ranges,
            styled_text: StyledText::new(text.clone()),
            text,
        }
    }

    fn paint_segment(
        window: &mut Window,
        cx: &mut App,
        left: Pixels,
        right: Pixels,
        baseline: Pixels,
    ) {
        if right <= left {
            return;
        }

        window.paint_quad(quad(
            Bounds::from_corners(point(left, baseline), point(right, baseline + px(1.))),
            px(0.),
            cx.theme().link,
            Edges::default(),
            gpui::transparent_black(),
            BorderStyle::default(),
        ));
    }

    fn paint_link_icon(window: &mut Window, cx: &mut App, x: Pixels, y: Pixels) {
        let color = cx.theme().link;
        let size = px(8.);
        let stroke = px(1.);
        let left = x;
        let top = y;

        window.paint_quad(quad(
            Bounds::from_corners(
                point(left + px(1.), top + px(2.)),
                point(left + size - px(2.), top + px(2.) + stroke),
            ),
            px(0.),
            color,
            Edges::default(),
            gpui::transparent_black(),
            BorderStyle::default(),
        ));
        window.paint_quad(quad(
            Bounds::from_corners(
                point(left + px(1.), top + px(5.)),
                point(left + size - px(2.), top + px(5.) + stroke),
            ),
            px(0.),
            color,
            Edges::default(),
            gpui::transparent_black(),
            BorderStyle::default(),
        ));
        window.paint_quad(quad(
            Bounds::from_corners(
                point(left + px(1.), top + px(2.)),
                point(left + px(1.) + stroke, top + size - px(1.)),
            ),
            px(0.),
            color,
            Edges::default(),
            gpui::transparent_black(),
            BorderStyle::default(),
        ));
        window.paint_quad(quad(
            Bounds::from_corners(
                point(left + size - px(2.), top + px(1.)),
                point(left + size - px(2.) + stroke, top + size - px(2.)),
            ),
            px(0.),
            color,
            Edges::default(),
            gpui::transparent_black(),
            BorderStyle::default(),
        ));
    }

    fn range_positions(
        &self,
        text_layout: &gpui::TextLayout,
        range: &Range<usize>,
        line_height: Pixels,
    ) -> Option<(Point<Pixels>, Point<Pixels>)> {
        let start_position = text_layout.position_for_index(range.start)?;
        let mut last_start = range.start;
        for (ix, _) in self.text[range.start..range.end].char_indices() {
            last_start = range.start + ix;
        }

        let last_position = text_layout.position_for_index(last_start)?;
        let end_position = text_layout
            .position_for_index(range.end)
            .filter(|pos| {
                pos.y == last_position.y
                    && pos.x > last_position.x
                    && pos.x - last_position.x <= line_height
            })
            .unwrap_or_else(|| point(last_position.x + line_height * 0.45, last_position.y));

        Some((start_position, end_position))
    }

    fn paint_link_text(
        &self,
        range: &Range<usize>,
        origin: Point<Pixels>,
        line_height: Pixels,
        window: &mut Window,
        cx: &mut App,
    ) {
        let url = &self.text[range.clone()];
        let text_style = window.text_style();
        let font_size = text_style.font_size.to_pixels(window.rem_size()) * 0.74;
        let icon_size = px(8.);
        let gap = px(3.);
        let text_origin = origin;
        let link_run = TextRun {
            len: url.len(),
            font: text_style.font(),
            color: cx.theme().link,
            background_color: None,
            underline: Some(UnderlineStyle {
                thickness: px(1.),
                color: Some(cx.theme().link),
                wavy: false,
            }),
            strikethrough: None,
        };
        let shaped_url = window.text_system().shape_line(
            url.to_string().into(),
            font_size,
            &[link_run.clone()],
            None,
        );

        Self::paint_link_icon(
            window,
            cx,
            origin.x - icon_size - gap,
            origin.y + (line_height - icon_size) * 0.5,
        );
        let _ = shaped_url.paint(text_origin, line_height, window, cx);
    }
}

impl IntoElement for LinkInlineOverlay {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for LinkInlineOverlay {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_element_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let base_style = window.text_style();
        let mut runs = Vec::new();
        let mut ix = 0;
        for range in &self.ranges {
            if ix < range.start {
                runs.push(base_style.clone().to_run(range.start - ix));
            }

            runs.push(
                base_style
                    .clone()
                    .highlight(HighlightStyle {
                        color: Some(cx.theme().link),
                        ..Default::default()
                    })
                    .to_run(range.len()),
            );
            ix = range.end;
        }
        if ix < self.text.len() {
            runs.push(base_style.to_run(self.text.len() - ix));
        }

        self.styled_text = StyledText::new(self.text.clone()).with_runs(runs);
        let (layout_id, _) =
            self.styled_text
                .request_layout(global_element_id, inspector_id, window, cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.styled_text
            .prepaint(id, inspector_id, bounds, &mut (), window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let text_layout = self.styled_text.layout().clone();
        let line_height = text_layout.line_height();

        for range in &self.ranges {
            let Some((start_position, end_position)) =
                self.range_positions(&text_layout, range, line_height)
            else {
                continue;
            };

            let cover = Bounds::from_corners(
                point(start_position.x - px(2.), start_position.y),
                point(end_position.x + px(2.), start_position.y + line_height),
            );
            window.paint_quad(fill(cover, cx.theme().background));

            if start_position.y == end_position.y {
                self.paint_link_text(range, start_position, line_height, window, cx);
            } else {
                Self::paint_segment(
                    window,
                    cx,
                    start_position.x,
                    bounds.right(),
                    start_position.y + line_height - px(2.),
                );

                let mut y = start_position.y + line_height;
                while y < end_position.y {
                    Self::paint_segment(
                        window,
                        cx,
                        bounds.left(),
                        bounds.right(),
                        y + line_height - px(2.),
                    );
                    y += line_height;
                }

                Self::paint_segment(
                    window,
                    cx,
                    bounds.left(),
                    end_position.x,
                    end_position.y + line_height - px(2.),
                );
            }
        }
    }
}

impl TeamWorkspacePanel {
    #[allow(dead_code)]
    fn update_chat_message_content(
        &mut self,
        session_id: &str,
        msg_idx: usize,
        content: String,
        thought_duration_secs: Option<f64>,
        cx: &mut Context<Self>,
    ) {
        if let Some(history) = self.chat_histories.get_mut(session_id) {
            if let Some(msg) = history.get_mut(msg_idx) {
                let is_final = thought_duration_secs.is_some();
                if is_final || msg.thought_duration_secs.is_none() {
                    msg.content = content.into();
                }
                if let Some(seconds) = thought_duration_secs {
                    msg.thought_duration_secs = Some(seconds);
                }
            }
        }
        self.rebuild_chat_display(session_id);
        if self.selected_session_id.as_deref() == Some(session_id) {
            let display_len = self
                .chat_display_rows
                .get(session_id)
                .map(|v| v.len())
                .unwrap_or(0);
            self.chat_list_state =
                gpui::ListState::new(display_len, gpui::ListAlignment::Bottom, px(200.));
        }
        cx.notify();
    }

    pub(crate) fn render_chat_column(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();

        let _active_team_id = self.selected_team_id.clone().or_else(|| {
            self.selected_instance_id.as_ref().and_then(|iid| {
                self.instances
                    .iter()
                    .find(|i| i.id == *iid)
                    .map(|i| i.team_id.clone())
            })
        });

        let title = if let Some(instance_id) = &self.selected_instance_id {
            let inst = self.instances.iter().find(|i| i.id == *instance_id);
            inst.map(|i| i.name.clone()).unwrap_or_else(|| {
                format!(
                    "Instance {}",
                    &instance_id[..std::cmp::min(8, instance_id.len())]
                )
            })
        } else if let Some(team_id) = &self.selected_team_id {
            self.teams
                .iter()
                .find(|t| t.id == *team_id)
                .map(|t| t.name.clone())
                .unwrap_or_else(|| "Team Chat".to_string())
        } else {
            "Team Chat".to_string()
        };

        div()
            .h_full()
            .w_full()
            .overflow_hidden()
            .flex()
            .flex_col()
            .bg(theme.background)
            .child(
                // Tabs
                div()
                    .w_full()
                    .h(px(36.))
                    .flex()
                    .items_center()
                    .border_b(px(1.))
                    .border_color(theme.border)
                    .bg(theme.background)
                    .child(
                        TabBar::new("chat_tabs")
                            .child(Tab::new().icon(IconName::SquareTerminal).label("Chat"))
                            .child(Tab::new().icon(IconName::Building2).label("Office"))
                            .selected_index(self.chat_active_tab)
                            .on_click({
                                let view = cx.entity().clone();
                                move |index, _window, cx| {
                                    view.update(cx, |this, cx| {
                                        this.chat_active_tab = *index;

                                        // Refresh native office state when switching tabs.
                                        this.sync_office_agents(cx);

                                        cx.notify();
                                    });
                                }
                            }),
                    ),
            )
            .child(
                // Top Header
                h_flex()
                    .w_full()
                    .h(px(36.))
                    .items_center()
                    .justify_between()
                    .px(px(12.))
                    .border_b(px(1.))
                    .border_color(theme.border)
                    .bg(theme.background)
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .items_center()
                            .child(
                                div()
                                    .w(px(24.))
                                    .h(px(24.))
                                    .rounded_md()
                                    .bg(gpui::red().opacity(0.2))
                                    .text_color(gpui::red())
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(IconName::User)
                            )
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_size(px(14.))
                                    .child(title)
                            )
                            .child(
                                h_flex()
                                    .gap(px(6.))
                                    .text_color(theme.muted_foreground)
                                    .text_size(px(12.))
                                    .child("\u{2014}")
                                    .child("Supervisor Chat")
                                    .child(div().text_color(gpui::green()).child("Active"))
                            )
                    )
                    .child(
                        h_flex()
                            .gap(px(4.))
                            .items_center()
                                    .child(
                                        Button::new("cross-team-target-top")
                                            .ghost()
                                            .icon(IconName::Inbox)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                let current = this.cross_team_target_instance_id.clone().unwrap_or_default();

                                                let mut options = Vec::new();
                                                let mut instance_ids = Vec::new();

                                                options.push(gpui::SharedString::from("None (Disable Cross-Team)"));
                                                instance_ids.push(String::new());

                                                for instance in &this.instances {
                                                    if Some(&instance.id) == this.selected_instance_id.as_ref() {
                                                        continue;
                                                    }
                                                    let team_name = this.teams.iter().find(|t| t.id == instance.team_id).map(|t| t.name.as_str()).unwrap_or("Unknown Team");
                                                    let label = format!("{} - {}", team_name, instance.name);
                                                    options.push(gpui::SharedString::from(label));
                                                    instance_ids.push(instance.id.clone());
                                                }

                                                let selected_idx = if !current.is_empty() {
                                                    instance_ids.iter().position(|id| id == &current)
                                                } else {
                                                    Some(0)
                                                };

                                                let select_state = cx.new(|cx| {
                                                    SelectState::new(options.clone(), selected_idx.map(IndexPath::new), window, cx)
                                                });

                                                let select_state2 = select_state.clone();
                                                let instance_ids2 = instance_ids.clone();
                                                let options2 = options.clone();
                                                let view = cx.entity().clone();

                                                window.open_dialog(cx, move |dialog, _window, _cx| {
                                                    dialog
                                                        .title("Coordinate: Cross-Team Target")
                                                        .w(px(520.))
                                                        .child(
                                                            gpui_component::form::v_form()
                                                                .gap(px(12.))
                                                                .py(px(8.))
                                                                .child(
                                                                    gpui_component::form::field()
                                                                        .label("Select Target Instance")
                                                                        .child(Select::new(&select_state2).placeholder("Select instance...")),
                                                                ),
                                                        )
                                                        .footer({
                                                            let select_state3 = select_state2.clone();
                                                            let instance_ids3 = instance_ids2.clone();
                                                            let options3 = options2.clone();
                                                            let view = view.clone();
                                                            move |_, _, _, _| {
                                                                vec![
                                                                    gpui_component::button::Button::new("cancel-cross-team")
                                                                        .label("Cancel")
                                                                        .on_click(|_, window, cx| {
                                                                            window.close_dialog(cx);
                                                                        })
                                                                        .into_any_element(),
                                                                    gpui_component::button::Button::new("save-cross-team")
                                                                        .primary()
                                                                        .label("Save")
                                                                        .on_click({
                                                                            let select_state4 = select_state3.clone();
                                                                            let instance_ids4 = instance_ids3.clone();
                                                                            let options_clone = options3.clone();
                                                                            let view = view.clone();
                                                                            move |_, window, cx| {
                                                                                let selected_label = select_state4.read(cx).selected_value().map(|s| s.to_string());
                                                                                let mut value = String::new();
                                                                                if let Some(label) = selected_label {
                                                                                    if let Some(pos) = options_clone.iter().position(|o| o.as_ref() == label) {
                                                                                        if let Some(id) = instance_ids4.get(pos) {
                                                                                            value = id.clone();
                                                                                        }
                                                                                    }
                                                                                }
                                                                                view.update(cx, |this: &mut super::TeamWorkspacePanel, cx| {
                                                                                    let instance_id = this.selected_instance_id.clone().unwrap_or_default();
                                                                                    let db = crate::AppState::global(cx).db.clone();
                                                                                    let old_target = this.cross_team_target_instance_id.clone().unwrap_or_default();
                                                                                    this.cross_team_target_instance_id = if value.trim().is_empty() { None } else { Some(value.clone()) };
                                                                                    if !instance_id.is_empty() {
                                                                                        let key = format!("cross_team_target_{}", instance_id);
                                                                                        let _ = db.set_setting(&key, value.trim());
                                                                                    }
                                                                                    if !old_target.trim().is_empty() && old_target != value {
                                                                                        let old_peer_key = format!("cross_team_peer_{}", old_target);
                                                                                        let _ = db.set_setting(&old_peer_key, "");
                                                                                    }
                                                                                    if !value.trim().is_empty() && !instance_id.is_empty() {
                                                                                        let peer_key = format!("cross_team_peer_{}", value);
                                                                                        let _ = db.set_setting(&peer_key, &instance_id);
                                                                                    }
                                                                                    if !instance_id.is_empty() {
                                                                                        let peer_key = format!("cross_team_peer_{}", instance_id);
                                                                                        this.cross_team_peer_instance_id = db
                                                                                            .get_setting(&peer_key)
                                                                                            .ok()
                                                                                            .flatten()
                                                                                            .filter(|v| !v.trim().is_empty());
                                                                                    }
                                                                                    cx.notify();
                                                                                });
                                                                                window.close_dialog(cx);
                                                                            }
                                                                        })
                                                                        .into_any_element(),
                                                                ]
                                                            }
                                                        })
                                                });
                                            }))
                                    )
                                    .child({
                                        let target_name = self
                                            .cross_team_target_instance_id
                                            .as_ref()
                                            .and_then(|id| self.instances.iter().find(|i| i.id == *id).map(|i| i.name.clone()))
                                            .or_else(|| self.cross_team_target_instance_id.clone())
                                            .unwrap_or_default();
                                        let peer_name = self
                                            .cross_team_peer_instance_id
                                            .as_ref()
                                            .and_then(|id| self.instances.iter().find(|i| i.id == *id).map(|i| i.name.clone()))
                                            .or_else(|| self.cross_team_peer_instance_id.clone())
                                            .unwrap_or_default();
                                        let mut row = h_flex().gap(px(6.)).items_center();
                                        if !peer_name.is_empty() {
                                            row = row.child(
                                                div()
                                                    .px(px(8.))
                                                    .py(px(2.))
                                                    .rounded_full()
                                                    .bg(theme.secondary)
                                                    .text_size(px(11.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(format!("<- {}", peer_name)),
                                            );
                                        }
                                        if !target_name.is_empty() {
                                            row = row.child(
                                                div()
                                                    .px(px(8.))
                                                    .py(px(2.))
                                                    .rounded_full()
                                                    .bg(theme.secondary)
                                                    .text_size(px(11.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(format!("-> {}", target_name)),
                                            );
                                        }
                                        row
                                    })
                                    .child(
                                        Button::new("open-cross-team-cases")
                                            .ghost()
                                            .label("Cases")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                let instance_id = this.selected_instance_id.clone().unwrap_or_default();
                                                if instance_id.is_empty() {
                                                    return;
                                                }
                                                let db = crate::AppState::global(cx).db.clone();
                                                let cases = Arc::new(db.list_cross_team_cases(&instance_id, 100).unwrap_or_default());
                                                let view = cx.entity().clone();
                                                window.open_sheet_at(gpui_component::Placement::Right, cx, move |sheet, _window, cx| {
                                                    let theme = cx.theme().clone();
                                                    sheet
                                                        .title("Cross-team Cases")
                                                        .size(px(520.))
                                                        .child(
                                                            gpui_component::v_flex()
                                                                .w_full()
                                                                .h_full()
                                                                .p_4()
                                                                .gap_2()
                                                                .overflow_y_scrollbar()
                                                                .child({
                                                                    let mut col = gpui_component::v_flex().w_full().gap_2();
                                                                    if cases.is_empty() {
                                                                        col = col.child(div().text_sm().text_color(theme.muted_foreground).child("No cases yet."));
                                                                    } else {
                                                                        for (idx, c) in cases.iter().enumerate() {
                                                                            let cid = c.correlation_id.clone();
                                                                            let summary = c.summary.clone();
                                                                            let latest = c.latest_event_type.clone();
                                                                            let db2 = db.clone();
                                                                            let view2 = view.clone();
                                                                            let btn = gpui_component::button::Button::new(("case", idx))
                                                                                .ghost()
                                                                                .on_click({
                                                                                        let cid2 = cid.clone();
                                                                                        let summary2 = summary.clone();
                                                                                        let latest2 = latest.clone();
                                                                                        move |_, window, cx| {
                                                                                            view2.update(cx, |this: &mut super::TeamWorkspacePanel, cx| {
                                                                                                this.selected_cross_team_case_id = Some(cid2.clone());
                                                                                                cx.notify();
                                                                                            });
                                                                                            let cid_for_sheet = cid2.clone();
                                                                                            let summary_for_sheet = summary2.clone();
                                                                                            let latest_for_sheet = latest2.clone();
                                                                                            let events = Arc::new(db2.list_cross_team_case_events(&cid2, 500).unwrap_or_default());
                                                                                            window.open_sheet_at(gpui_component::Placement::Right, cx, move |sheet, _window, cx| {
                                                                                                let theme = cx.theme().clone();
                                                                                                let step_idx = {
                                                                                                    let t = latest_for_sheet.as_str();
                                                                                                    match t {
                                                                                                        "ACK_RECEIVED" => 0,
                                                                                                        "READBACK_CONFIRMED" => 1,
                                                                                                        "PLAN_CREATED" | "SUBTASKS_DISPATCHED" => 2,
                                                                                                        "PARTIAL_RESULT" => 3,
                                                                                                        "REVIEW_REQUEST" | "REVIEW_RESPONSE" => 4,
                                                                                                        "FINAL_RESULT" | "CONSENSUS_REACHED" => 5,
                                                                                                        _ => 0,
                                                                                                    }
                                                                                                };
                                                                                                let step_labels = ["Received", "Readback", "Plan", "In Progress", "Review", "Done"];
                                                                                                sheet
                                                                                                    .title("Case Detail")
                                                                                                    .size(px(720.))
                                                                                                    .child(
                                                                                                        gpui_component::v_flex()
                                                                                                            .w_full()
                                                                                                            .h_full()
                                                                                                            .min_w_0()
                                                                                                            .overflow_hidden()
                                                                                                            .p_4()
                                                                                                            .gap_3()
                                                                                                            .child(div().flex_none().min_w_0().text_sm().whitespace_normal().line_height(gpui::relative(1.35)).text_color(theme.muted_foreground).child(format!("correlation_id: {}", cid_for_sheet)))
                                                                                                            .child(div().flex_none().min_w_0().text_sm().whitespace_normal().line_height(gpui::relative(1.45)).child(summary_for_sheet.clone()))
                                                                                                            .child({
                                                                                                                let mut row = h_flex().gap(px(10.)).items_center().w_full().min_w_0().flex_none();
                                                                                                                for (i, label) in step_labels.iter().enumerate() {
                                                                                                                    let active = i <= step_idx;
                                                                                                                    let dot = div()
                                                                                                                        .w(px(10.))
                                                                                                                        .h(px(10.))
                                                                                                                        .rounded_full()
                                                                                                                        .bg(if active { theme.accent } else { theme.border });
                                                                                                                    row = row.child(
                                                                                                                        h_flex()
                                                                                                                            .gap(px(6.))
                                                                                                                            .items_center()
                                                                                                                            .child(dot)
                                                                                                                            .child(div().text_xs().text_color(if active { theme.foreground } else { theme.muted_foreground }).child(*label)),
                                                                                                                    );
                                                                                                                    if i < step_labels.len() - 1 {
                                                                                                                        row = row.child(div().h(px(1.)).flex_1().bg(theme.border));
                                                                                                                    }
                                                                                                                }
                                                                                                                row
                                                                                                            })
                                                                                                            .child(div().flex_none().text_sm().font_weight(gpui::FontWeight::SEMIBOLD).child("Events"))
                                                                                                            .child({
                                                                                                                let event_list_state = gpui::ListState::new(events.len(), gpui::ListAlignment::Top, px(96.));
                                                                                                                let events_for_list = events.clone();
                                                                                                                let theme_for_list = theme.clone();
                                                                                                                div()
                                                                                                                    .flex_1()
                                                                                                                    .min_h_0()
                                                                                                                    .min_w_0()
                                                                                                                    .vertical_scrollbar(&event_list_state)
                                                                                                                    .child(
                                                                                                                        gpui::list(
                                                                                                                            event_list_state,
                                                                                                                            move |ix, _window, _cx| {
                                                                                                                                let Some(e) = events_for_list.get(ix) else {
                                                                                                                                    return div().into_any_element();
                                                                                                                                };

                                                                                                                                gpui_component::v_flex()
                                                                                                                                    .w_full()
                                                                                                                                    .min_w_0()
                                                                                                                                    .mb_3()
                                                                                                                                    .p_3()
                                                                                                                                    .gap_2()
                                                                                                                                    .rounded_md()
                                                                                                                                    .bg(theme_for_list.secondary)
                                                                                                                                    .child(div().min_w_0().text_xs().whitespace_normal().line_height(gpui::relative(1.35)).text_color(theme_for_list.muted_foreground).child(format!("{} - {}", e.created_at, e.event_type)))
                                                                                                                                    .child(div().min_w_0().text_sm().whitespace_normal().line_height(gpui::relative(1.45)).child(e.summary.clone()))
                                                                                                                                    .into_any_element()
                                                                                                                            },
                                                                                                                        )
                                                                                                                        .size_full(),
                                                                                                                    )
                                                                                                            }),
                                                                                                    )
                                                                                            });
                                                                                        }
                                                                                    })
                                                                                .label(latest.clone());
                                                                            col = col.child(
                                                                                gpui_component::v_flex()
                                                                                    .w_full()
                                                                                    .gap_1()
                                                                                    .child(btn)
                                                                                    .child(div().text_xs().text_color(theme.muted_foreground).child(summary)),
                                                                            );
                                                                        }
                                                                    }
                                                                    col
                                                                }),
                                                        )
                                                })
                                            }))
                                    )
                                    .child(
                                        Button::new("new-conversation-top")
                                            .ghost()
                                            .icon(IconName::Plus)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                if let Some(instance_id) = &this.selected_instance_id {
                                                    let agent_id = crate::AppState::global(cx).db.get_instance_agents(instance_id)
                                                        .ok()
                                                        .and_then(|ids| ids.first().cloned());
                                                    if let Some(agent_id) = agent_id {
                                                        if crate::AppState::global(cx).db.create_session_for_instance(instance_id, &agent_id).is_ok() {
                                                            let sessions = crate::AppState::global(cx).db.list_sessions_for_instance(instance_id).unwrap_or_default();
                                                            this.sessions_for_instance = sessions.clone();
                                                            if let Some(s) = sessions.first() {
                                                                this.selected_session_id = Some(s.id.clone());
                                                                this.instance_active_session.insert(instance_id.clone(), s.id.clone());
                                                                let msgs = crate::AppState::global(cx).db.get_conversation_turns(&s.id).unwrap_or_default();
                                                                this.chat_histories.insert(s.id.clone(), msgs);
                                                                this.rebuild_chat_display(&s.id);
                                                                this.refresh_pending_chat_action(cx);
                                                                let history_len = this.chat_display_rows.get(&s.id).map(|h| h.len()).unwrap_or(0);
                                                                this.chat_list_state = gpui::ListState::new(history_len, gpui::ListAlignment::Bottom, px(200.));
                                                            }
                                                            cx.notify();
                                                        }
                                                    }
                                                }
                                            }))
                                    )
                                    .child(
                                        Button::new("toggle-history")
                                            .ghost()
                                            .icon(Icon::empty().path("icons/history.svg").size_4())
                                            .tooltip("Past Session")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.show_history_sheet = !this.show_history_sheet;
                                                cx.notify();
                                            }))
                                    )
                                    .when(
                                        self.cross_team_target_instance_id.is_some()
                                            || self.cross_team_peer_instance_id.is_some(),
                                        |header| {
                                            header.child(
                                                Button::new("open-combined-office")
                                                    .ghost()
                                                    .small()
                                                    .icon(IconName::Building2)
                                                    .label("Combined Office")
                                                    .tooltip("Show Combined Office")
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.chat_active_tab = 1;
                                                        this.office_combined_mode = true;
                                                        this.office_quick_chat_agent_id = None;
                                                        this.office_state.dragged_agent_idx = None;
                                                        this.sync_office_agents(cx);
                                                        cx.notify();
                                                    })),
                                            )
                                        },
                                    )
                                    .when_some(
                                        self.available_iflow_run_id.clone(),
                                        |header, run_id| {
                                            header.child(
                                                Button::new("view-run-workspace")
                                                    .ghost()
                                                    .small()
                                                    .label("Run")
                                                    .tooltip("Open the workspace for this run")
                                                    .on_click(move |_, _, cx| {
                                                        let db = crate::AppState::global(cx).db.clone();
                                                        let selected = crate::AppState::global(cx)
                                                            .selected_orchestration_run_id
                                                            .clone();
                                                        let active_panel = crate::AppState::global(cx)
                                                            .active_panel
                                                            .clone();
                                                        let _ = db.set_setting(
                                                            "orchestration_selected_run_id",
                                                            &run_id,
                                                        );
                                                        let selected_run_id = run_id.clone();
                                                        selected.update(cx, move |current, cx| {
                                                            *current = Some(selected_run_id);
                                                            cx.notify();
                                                        });
                                                        active_panel.update(cx, |page, cx| {
                                                            *page = "orchestration".to_string();
                                                            cx.notify();
                                                        });
                                                    }),
                                            )
                                        },
                                    )
                                    .when_some(
                                        self.available_iflow_run_id.clone(),
                                        |header, run_id| {
                                            header.child(
                                                Button::new("view-iflow-run")
                                                    .ghost()
                                                    .small()
                                                    // .icon(IconName::GalleryVerticalEnd)
                                                    .icon(Icon::empty().path("icons/flow.svg").size_4())
                                                    .label("iFlow")
                                                    .tooltip("View the validated flow for this run")
                                                    .on_click(move |_, _, cx| {
                                                        let db =
                                                            crate::AppState::global(cx).db.clone();
                                                        let selected = crate::AppState::global(cx)
                                                            .selected_iflow_run_id
                                                            .clone();
                                                        let active_panel =
                                                            crate::AppState::global(cx)
                                                                .active_panel
                                                                .clone();
                                                        let _ = db.set_setting(
                                                            "iflow_selected_run_id",
                                                            &run_id,
                                                        );
                                                        let selected_run_id = run_id.clone();
                                                        selected.update(cx, move |current, cx| {
                                                            *current = Some(selected_run_id);
                                                            cx.notify();
                                                        });
                                                        active_panel.update(cx, |page, cx| {
                                                            *page = "iflow_builder".to_string();
                                                            cx.notify();
                                                        });
                                                    }),
                                            )
                                        },
                                    )
                    )
            )
            .child(
                if self.chat_active_tab == 1 {
                    // Native Office View — rendered purely in Rust/GPUI
                    self.sync_office_agents(cx);
                    self.render_office_view(window, cx).into_any_element()
                } else {
                    // Chat View
                    let mut form_header = div()
                        .flex()
                        .gap_2()
                        .p_2()
                        .justify_between()
                        .items_center()
                        .child(
                            div().flex().gap_2().items_center().child(
                                Button::new("add-file")
                                    .icon(Icon::empty().path("icons/attachment.svg").size_4())
                                    .ghost()
                                    .mr_1()
                                    .on_click(cx.listener(|_this, _, _, cx| {
                                        let view = cx.entity().clone();
                                        cx.spawn(async move |_, cx| {
                                            if let Some(file) = rfd::AsyncFileDialog::new().pick_file().await {
                                                let path = file.path().to_string_lossy().to_string();
                                                let _ = cx.update(|cx| {
                                                    view.update(cx, |this: &mut super::TeamWorkspacePanel, cx| {
                                                        if !this.attached_files.contains(&path) {
                                                            this.attached_files.push(path);
                                                            cx.notify();
                                                        }
                                                    });
                                                });
                                            }
                                        }).detach();
                                    }))
                            )

                        );

                    if !self.attached_files.is_empty() {
                        form_header = form_header.child(gpui_component::divider::Divider::vertical());

                        let mut files_container = div().flex().gap_2().flex_wrap().w_full();
                        for (idx, path) in self.attached_files.iter().enumerate() {
                            let p = std::path::Path::new(path);
                            let file_name = p.file_name().and_then(|s| s.to_str()).unwrap_or(path).to_string();
                            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_uppercase();
                            let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
                            let size_kb = size as f64 / 1024.0;

                            let is_image = ["PNG", "JPG", "JPEG", "GIF", "WEBP"].contains(&ext.as_str());

                            let icon_box = div()
                                .w(px(32.))
                                .h(px(32.))
                                .bg(theme.background)
                                .rounded_sm()
                                .overflow_hidden()
                                .flex()
                                .justify_center()
                                .items_center();

                            let icon_child = if is_image {
                                use gpui::StyledImage;
                                let path_buf = std::path::PathBuf::from(path);
                                icon_box.child(gpui::img(path_buf).w_full().h_full().object_fit(gpui::ObjectFit::Cover))
                            } else {
                                icon_box.child(Icon::new(IconName::File).size(px(14.)).text_color(theme.muted_foreground))
                            };

                            files_container = files_container.child(
                                div()
                                    .relative()
                                    .group(format!("file-upload-{}", idx))
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .p_1()
                                    .pr_3() // Give some space on right
                                    .rounded_md()
                                    .bg(theme.secondary)
                                    .border_1()
                                    .border_color(theme.border)
                                    .child(icon_child)
                                    .child(
                                        div().flex_col()
                                            .child(div().text_xs().text_color(theme.foreground).child(file_name))
                                            .child(div().text_xs().text_color(theme.muted_foreground).child(format!("{} - {:.1} KB", ext, size_kb)))
                                    )
                                    .child(
                                        div()
                                            .absolute()
                                            .top(px(-6.))
                                            .right(px(-6.))
                                            .w(px(16.))
                                            .h(px(16.))
                                            .flex()
                                            .justify_center()
                                            .items_center()
                                            .bg(theme.border)
                                            .rounded_full()
                                            .cursor_pointer()
                                            .invisible()
                                            .group_hover(format!("file-upload-{}", idx), |s| s.visible().bg(gpui::rgba(0x000000aa)))
                                            .child(Icon::new(IconName::Close).size(px(10.)).text_color(theme.muted_foreground))
                                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                                this.attached_files.remove(idx);
                                                cx.notify();
                                            }))
                                    )
                            );
                        }
                        form_header = form_header.child(files_container);
                    }

                    let workspace_label = if let Some(path) = &self.workspace_path {
                        let p = std::path::Path::new(path);
                        p.file_name().and_then(|s| s.to_str()).unwrap_or(path).to_string()
                    } else {
                        "Select Folder".to_string()
                    };

                    let form_footer = div()
                        .flex()
                        .gap_2()
                        .p_2()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(
                                    div()
                                        .relative()
                                        .child(
                                            div()
                                                .id("workspace-selector")
                                                .flex()
                                                .justify_start()
                                                .gap_2()
                                                .px_2()
                                                .py_1()
                                                .rounded_md()
                                                .bg(theme.secondary)
                                                .border_1()
                                                .border_color(theme.border)
                                                .items_center()
                                                .cursor_pointer()
                                                .hover(|s| s.bg(theme.border))
                                                .on_click(cx.listener(|this, _, _window, cx| {
                                                    this.is_workspace_dropdown_open = !this.is_workspace_dropdown_open;
                                                    if this.is_workspace_dropdown_open {
                                                        let db = crate::AppState::global(cx).db.clone();
                                                        if let Ok(recents) = db.get_recent_workspaces() {
                                                            this.recent_workspaces = recents;
                                                        }
                                                    }

                                                    // Refresh native office state after workspace menu changes.
                                                    this.sync_office_agents(cx);

                                                    cx.notify();
                                                }))
                                                .child(Icon::new(IconName::FolderOpen).size(px(14.)).text_color(theme.muted_foreground))
                                                .child(div().text_sm().child(workspace_label))
                                                .child(Icon::new(IconName::ChevronDown).size(px(12.)).text_color(theme.muted_foreground))
                                        )
                                        .child(
                                            if self.is_workspace_dropdown_open {
                                                // Create a list of dummy recent folders + Create Project button
                                                let mut recent_list = div().flex_col().gap_1();

                                                for r in &self.recent_workspaces {
                                                    let r_str = r.clone();
                                                    let display_name = std::path::Path::new(&r_str)
                                                        .file_name()
                                                        .and_then(|s| s.to_str())
                                                        .unwrap_or(&r_str)
                                                        .to_string();

                                                    let is_selected = self.workspace_path.as_ref().is_some_and(|p| p == &r_str);

                                                    recent_list = recent_list.child(
                                                        div()
                                                            .id(gpui::ElementId::Name(format!("recent-{}", display_name).into()))
                                                            .flex()
                                                            .items_center()
                                                            .justify_between()
                                                            .px_2()
                                                            .py_1()
                                                            .rounded_md()
                                                            .hover(|s| s.bg(theme.secondary))
                                                            .cursor_pointer()
                                                            .on_click({
                                                                let r_str_clone = r_str.clone();
                                                                cx.listener(move |this, _, _, cx| {
                                                                    if let Some(instance_id) = &this.selected_instance_id {
                                                                        let path = r_str_clone.clone();
                                                                        let key = format!("workspace_{}", instance_id);
                                                                        let _ = crate::AppState::global(cx).db.set_setting(&key, &path);
                                                                        this.workspace_path = Some(path);
                                                                        this.is_workspace_dropdown_open = false;

                                                                        // Refresh native office state after selecting workspace.
                                                                        this.sync_office_agents(cx);

                                                                        cx.notify();
                                                                    }
                                                                })
                                                            })
                                                            .child(
                                                                div().flex().items_center().gap_2()
                                                                    .child(Icon::new(IconName::Folder).size(px(14.)).text_color(theme.muted_foreground))
                                                                    .child(div().text_sm().child(display_name))
                                                            )
                                                            .child(
                                                                if is_selected {
                                                                    Icon::new(IconName::Check).size(px(14.)).text_color(gpui::green())
                                                                } else {
                                                                    Icon::new(IconName::Check).size(px(14.)).text_color(gpui::transparent_black())
                                                                }
                                                            )
                                                    );
                                                }

                                                div()
                                                    .absolute()
                                                    .bottom(px(32.))
                                                    .left(px(0.))
                                                    .w(px(250.))
                                                    .bg(theme.background)
                                                    .border_1()
                                                    .border_color(theme.border)
                                                    .rounded_md()
                                                    .shadow_lg()
                                                    .p_2()
                                                    .flex_col()
                                                    .gap_2()
                                                    .child(div().text_sm().text_color(theme.muted_foreground).child("Recent"))
                                                    .child(recent_list)
                                                    .child(gpui_component::divider::Divider::horizontal().my_1())
                                                    .child(
                                                        div()
                                                            .id("create-project-btn")
                                                            .flex()
                                                            .items_center()
                                                            .gap_2()
                                                            .px_2()
                                                            .py_1()
                                                            .rounded_md()
                                                            .hover(|s| s.bg(theme.secondary))
                                                            .cursor_pointer()
                                                            .on_click(cx.listener(|this, _, _window, cx| {
                                                                // Just open the folder picker for now
                                                                this.is_workspace_dropdown_open = false;
                                                                cx.notify();

                                                                if let Some(instance_id) = &this.selected_instance_id {
                                                                    let db = crate::AppState::global(cx).db.clone();
                                                                    let instance_id_clone = instance_id.clone();
                                                                    let view = cx.entity().clone();
                                                                    let start_dir = this.workspace_path.clone();

                                                                    cx.spawn(async move |_, cx| {
                                                                        let mut dialog = rfd::AsyncFileDialog::new()
                                                                            .set_title("Select Workspace Folder");
                                                                        if let Some(ref dir) = start_dir {
                                                                            dialog = dialog.set_directory(dir);
                                                                        }
                                                                        if let Some(folder) = dialog.pick_folder().await {
                                                                            let path = folder.path().to_string_lossy().to_string();
                                                                            let key = format!("workspace_{}", instance_id_clone);
                                                                            let _ = db.set_setting(&key, &path);
                                                                            let _ = cx.update(|cx| {
                                                                                view.update(cx, |this: &mut super::TeamWorkspacePanel, cx| {
                                                                                    this.workspace_path = Some(path);
                                                                                    cx.notify();
                                                                                });
                                                                            });
                                                                        }
                                                                    }).detach();
                                                                }
                                                            }))
                                                            .child(Icon::new(IconName::Plus).size(px(14.)).text_color(theme.muted_foreground))
                                                            .child(div().text_sm().child("Create project"))
                                                    )
                                            } else {
                                                div()
                                            }
                                        )
                                )
                                .child(gpui_component::divider::Divider::vertical().h(px(16.)))
                                .child(
                                    div()
                                        .id("slash-cmd")
                                        .cursor_pointer()
                                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                            this.is_slash_dropdown_open = !this.is_slash_dropdown_open;
                                            this.slash_command_selection_index = 0;
                                            this.slash_command_active_query =
                                                this.active_slash_command_query(cx);
                                            if this.is_slash_dropdown_open {
                                                this.is_workspace_dropdown_open = false;
                                            } else {
                                                this.slash_command_active_query = None;
                                            }
                                            cx.notify();
                                        }))
                                        .child(div().text_sm().font_weight(gpui::FontWeight::BOLD).text_color(theme.muted_foreground).child("/"))
                                )
                                .child(
                                    div()
                                        .id("attach-btn")
                                        .cursor_pointer()
                                        .on_click(cx.listener(|_this, _, _window, cx| {
                                            let view = cx.entity().clone();
                                            cx.spawn(async move |_, cx| {
                                                if let Some(file) = rfd::AsyncFileDialog::new().pick_file().await {
                                                    let _ = cx.update(|cx| {
                                                        view.update(cx, |this: &mut super::TeamWorkspacePanel, cx| {
                                                            this.attached_files.push(file.path().to_string_lossy().to_string());
                                                            cx.notify();
                                                        });
                                                    });
                                                }
                                            }).detach();
                                        }))
                                        .child(Icon::empty().path("icons/attachment.svg").size_4().text_color(theme.muted_foreground))
                                )
                        )
                        .child(
                            Button::new("send-chat")
                                .rounded_full()
                                .bg(theme.accent)
                                .icon(if self.is_generating { IconName::Close } else { IconName::ArrowUp })
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.handle_send_chat(window, cx);
                                }))
                        );

                    let active_slash_query = self.active_slash_command_query(cx);
                    let show_slash_dropdown = active_slash_query.is_some();
                    let slash_query = active_slash_query.unwrap_or_default();
                    let composer_key_context = if show_slash_dropdown {
                        "TeamWorkspaceSlashCommands"
                    } else {
                        "TeamWorkspaceChatComposer"
                    };
                    let slash_dropdown = if show_slash_dropdown {
                        div()
                            .w_full()
                            .mb(px(8.))
                            .child(self.render_slash_command_dropdown(&slash_query, cx))
                            .into_any_element()
                    } else {
                        div().into_any_element()
                    };

                    let form = div()
                        .relative()
                        .flex()
                        .flex_col()
                        .justify_between()
                        .rounded_2xl()
                        .border_1()
                        .border_color(theme.border.opacity(0.8))
                        .bg(theme.popover)
                        .h(px(220.))
                        .shadow_lg()
                        .w_full()
                        .child(
                            div().flex().flex_col().child(form_header)
                            .child(
                                h_flex().w_full().items_center().px_2().py_1()
                                    .children(self.selected_slash_command.map(|cmd| {
                                        let definition = cmd.definition();
                                        let bg = Self::slash_command_color(
                                            definition.tone,
                                            theme.accent,
                                        );
                                        let icon = definition.icon.clone();
                                        div().flex().items_center().gap_1()
                                            .id("active-slash-cmd-pill")
                                            .bg(bg.opacity(0.2))
                                            // .text_color(bg)
                                            .px_2().py_1().rounded_md()
                                            .mr_2()
                                            .cursor_pointer()
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.selected_slash_command = None;
                                                this.slash_command_selection_index = 0;
                                                this.slash_command_active_query = None;
                                                cx.notify();
                                            }))
                                            .child(Icon::new(icon).size(px(14.)))
                                            .child(div().text_sm().font_weight(gpui::FontWeight::MEDIUM).child(definition.label))
                                    }))
                                    .child(
                                        {
                                            let input_text =
                                                self.chat_input_state.read(cx).text().to_string();
                                            let ranges = url_ranges(&input_text);

                                            div()
                                                .relative()
                                                .flex_1()
                                                .min_w_0()
                                                .key_context(composer_key_context)
                                                .on_action(cx.listener(Self::on_chat_composer_confirm))
                                                .on_action(cx.listener(Self::on_slash_command_previous))
                                                .on_action(cx.listener(Self::on_slash_command_next))
                                                .child(
                                                    gpui_component::input::Input::new(
                                                        &self.chat_input_state,
                                                    )
                                                    .appearance(false),
                                                )
                                                .when(!ranges.is_empty(), |this| {
                                                    this.child(
                                                        div()
                                                            .absolute()
                                                            .left(px(12.))
                                                            .right(px(12.))
                                                            .top(px(8.))
                                                            .bottom(px(0.))
                                                            .child(LinkInlineOverlay::new(
                                                                input_text,
                                                                ranges,
                                                            )),
                                                    )
                                                })
                                        }
                                    )
                            )
                        )
                        .child(form_footer);

                    h_flex()
                        .flex_1()
                        .size_full()
                        .child(
                            div()
                                .flex_1()
                                .v_flex()
                                .size_full()
                                .bg(theme.background)
                                .child(
                                    div()
                                        .p_2()
                                        .v_flex()
                                        .size_full()
                                        .child(
                                            div().p_2().pb(px(24.)).size_full().flex().child(
                                                gpui::list(
                                                    self.chat_list_state.clone(),
                                                    cx.processor(|this: &mut Self, ix, window, cx| this.render_entry(ix, window, cx)),
                                                )
                                                .size_full(),
                                            ),
                                        )
                                        .child(self.render_cached_pending_action_summary(cx))
                                        .child(slash_dropdown)
                                        .child(form)
                                )
                        )
                        .when(self.show_history_sheet, |d| {
                            let mut sessions_list = div().flex().flex_col().gap(px(4.));
                            for s in &self.sessions_for_instance {
                                let session_id = s.id.clone();
                                let is_selected = self.selected_session_id.as_deref() == Some(session_id.as_str());
                                let label = format_session_label(s);
                                sessions_list = sessions_list.child(
                                    gpui_component::button::Button::new(gpui::SharedString::from(format!("session-{}", session_id)))
                                        .small()
                                        .ghost()
                                        .when(is_selected, |b| b.primary())
                                        .label(label)
                                        .on_click(cx.listener({
                                            let session_id = session_id.clone();
                                            move |this, _, _, cx| {
                                                this.selected_session_id = Some(session_id.clone());
                                                if let Some(instance_id) = this.selected_instance_id.clone() {
                                                    this.instance_active_session
                                                        .insert(instance_id, session_id.clone());
                                                }
                                                let db = crate::AppState::global(cx).db.clone();
                                                let msgs = db
                                                    .get_conversation_turns(&session_id)
                                                    .unwrap_or_default();
                                                this.chat_histories.insert(session_id.clone(), msgs);
                                                this.rebuild_chat_display(&session_id);
                                                this.refresh_pending_chat_action(cx);
                                                let history_len = this
                                                    .chat_display_rows
                                                    .get(&session_id)
                                                    .map(|h| h.len())
                                                    .unwrap_or(0);
                                                this.chat_list_state = gpui::ListState::new(
                                                    history_len,
                                                    gpui::ListAlignment::Bottom,
                                                    px(200.),
                                                );
                                                cx.notify();
                                            }
                                        }))
                                );
                            }

                            d.child(
                                div()
                                    .w(px(250.))
                                    .h_full()
                                    .border_l(px(1.))
                                    .border_color(theme.border)
                                    .bg(theme.secondary.opacity(0.3))
                                    .p_2()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_size(px(14.))
                                            .mb(px(8.))
                                            .child("Session History")
                                    )
                                    .child(
                                        div()
                                            .id("session-history-scroll")
                                            .flex_1()
                                            .overflow_y_scroll()
                                            .child(sessions_list)
                                    )
                            )
                        })
                        .into_any_element()
                }
            )
    }

    fn slash_command_color(tone: SlashCommandTone, accent: gpui::Hsla) -> gpui::Hsla {
        match tone {
            SlashCommandTone::Accent => accent,
            SlashCommandTone::Blue => gpui::blue(),
            SlashCommandTone::Green => gpui::green(),
            SlashCommandTone::Yellow => gpui::yellow(),
            SlashCommandTone::Red => gpui::red(),
        }
    }

    fn render_slash_command_dropdown(&self, query: &str, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let commands = filter_slash_commands(query);
        let selected_index = self
            .slash_command_selection_index
            .min(commands.len().saturating_sub(1));

        let mut list = div()
            .w_full()
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded_xl()
            .shadow_lg()
            .p_3()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.muted_foreground)
                    .mb_2()
                    .child("Commands"),
            );

        if commands.is_empty() {
            list = list.child(
                div()
                    .id("slash-no-match")
                    .w_full()
                    .px_2()
                    .py_2()
                    .rounded_md()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("Không tìm thấy command"),
            );
        } else {
            for (index, command) in commands.into_iter().enumerate() {
                let command_id = command.id;
                let color = Self::slash_command_color(command.tone, theme.accent);
                let icon = command.icon.clone();
                let row_id =
                    gpui::ElementId::Name(format!("slash-item-{}", command.trigger).into());
                list = list.child(
                    div()
                        .id(row_id)
                        .w_full()
                        .px_2()
                        .py_2()
                        .rounded_md()
                        .when(index == selected_index, |s| {
                            s.bg(theme.secondary.opacity(0.55))
                        })
                        .hover(|s| s.bg(theme.secondary))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.selected_slash_command = Some(command_id);
                            this.slash_command_selection_index = 0;
                            this.slash_command_active_query = None;
                            this.chat_input_state.update(cx, |state, cx| {
                                state.set_value("", window, cx);
                            });
                            this.is_slash_dropdown_open = false;
                            cx.notify();
                        }))
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .w(px(20.))
                                .h(px(20.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(color)
                                .child(Icon::new(icon).size(px(14.))),
                        )
                        .child(
                            div()
                                .w(px(76.))
                                .text_sm()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .child(format!("/{}", command.trigger)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(command.description),
                        ),
                );
            }
        }

        list.into_any_element()
    }

    fn active_slash_command_query(&self, cx: &Context<Self>) -> Option<String> {
        let input_text = self.chat_input_state.read(cx).text().to_string();
        let trimmed = input_text.trim();
        slash_query_from_input(&input_text).or_else(|| {
            if self.is_slash_dropdown_open && trimmed.is_empty() {
                Some(String::new())
            } else {
                None
            }
        })
    }

    pub(crate) fn sync_slash_command_query(&mut self, cx: &mut Context<Self>) {
        let active_query = self.active_slash_command_query(cx);
        let input_text = self.chat_input_state.read(cx).text().to_string();
        if active_query.is_none() && !input_text.trim().is_empty() && self.is_slash_dropdown_open {
            self.is_slash_dropdown_open = false;
        }
        if active_query != self.slash_command_active_query {
            self.slash_command_active_query = active_query;
            self.slash_command_selection_index = 0;
            cx.notify();
        }
    }

    pub(crate) fn move_slash_command_selection(
        &mut self,
        direction: isize,
        cx: &mut Context<Self>,
    ) {
        let Some(query) = self.active_slash_command_query(cx) else {
            return;
        };
        let commands = filter_slash_commands(&query);
        if commands.is_empty() {
            self.slash_command_selection_index = 0;
            cx.notify();
            return;
        }

        let len = commands.len();
        let current = self.slash_command_selection_index.min(len - 1);
        self.slash_command_selection_index = if direction < 0 {
            if current == 0 {
                len - 1
            } else {
                current - 1
            }
        } else {
            (current + 1) % len
        };
        cx.notify();
    }

    pub(crate) fn confirm_slash_command_from_input(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.selected_slash_command.is_some() {
            return false;
        }

        let Some(query) = self.active_slash_command_query(cx) else {
            return false;
        };

        let commands = filter_slash_commands(&query);
        let selected_index = self
            .slash_command_selection_index
            .min(commands.len().saturating_sub(1));
        let Some(command) = commands.get(selected_index).cloned() else {
            self.is_slash_dropdown_open = true;
            self.slash_command_selection_index = 0;
            window.push_notification(
                (
                    gpui_component::notification::NotificationType::Info,
                    gpui::SharedString::from(
                        "Không tìm thấy slash command. Gõ /help để xem danh sách command.",
                    ),
                ),
                cx,
            );
            cx.notify();
            return true;
        };

        let exact_match = !query.is_empty()
            && (command.trigger == query
                || command.label.eq_ignore_ascii_case(&query)
                || command.aliases.iter().any(|alias| *alias == query));

        if exact_match && !command.requires_argument {
            self.selected_slash_command = None;
            self.is_slash_dropdown_open = false;
            self.slash_command_selection_index = 0;
            self.slash_command_active_query = None;
            let command_text = format!("/{}", command.trigger);
            self.chat_input_state.update(cx, |state, cx| {
                state.set_value(&command_text, window, cx);
            });
            cx.notify();
            self.handle_send_chat(window, cx);
            return true;
        }

        self.selected_slash_command = Some(command.id);
        self.is_slash_dropdown_open = false;
        self.slash_command_selection_index = 0;
        self.slash_command_active_query = None;
        self.chat_input_state.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
        cx.notify();
        true
    }

    /// Push a chat message bubble onto an agent in the native office canvas.
    pub(crate) fn push_office_chat_message(
        &mut self,
        agent_id: &str,
        text: &str,
        _is_self: bool,
        _agent_name: &str,
        _cx: &gpui::App,
    ) {
        self.office_state.push_message(agent_id, text);
    }

    fn persist_initial_iflow_for_run(
        db: &Arc<dyn crate::core::traits::database::DatabasePort>,
        run_id: &str,
        team_id: &str,
        instance_id: &str,
        agent_id: &str,
        goal: &str,
    ) -> Result<String, String> {
        let workflow_id = uuid::Uuid::new_v4().to_string();
        let task_node_id = "goal".to_string();
        let mut nodes = std::collections::HashMap::new();
        nodes.insert(
            "start".to_string(),
            crate::application::iflow_engine::nodes::Node {
                id: "start".to_string(),
                name: "Start".to_string(),
                node_type: crate::application::iflow_engine::nodes::NodeType::Start,
                next_nodes: vec![task_node_id.clone()],
            },
        );
        nodes.insert(
            task_node_id.clone(),
            crate::application::iflow_engine::nodes::Node {
                id: task_node_id,
                name: "User Goal".to_string(),
                node_type: crate::application::iflow_engine::nodes::NodeType::AgentTask {
                    agent_id: agent_id.to_string(),
                    instruction: goal.to_string(),
                    input_vars: Vec::new(),
                    output_var: Some("result".to_string()),
                },
                next_nodes: vec!["end".to_string()],
            },
        );
        nodes.insert(
            "end".to_string(),
            crate::application::iflow_engine::nodes::Node {
                id: "end".to_string(),
                name: "End".to_string(),
                node_type: crate::application::iflow_engine::nodes::NodeType::End,
                next_nodes: Vec::new(),
            },
        );
        let workflow = crate::application::iflow_engine::engine::Workflow {
            id: workflow_id.clone(),
            name: format!(
                "Goal Flow {}",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
            ),
            version: "1.0".to_string(),
            nodes,
            start_node_id: "start".to_string(),
            team_id: Some(team_id.to_string()),
            instance_id: Some(instance_id.to_string()),
        };
        crate::application::iflow_engine::engine::WorkflowEngine::validate_workflow(&workflow)?;
        let definition = serde_json::to_string(&workflow).map_err(|error| error.to_string())?;
        db.upsert_workflow(&crate::core::models::WorkflowRecord {
            id: workflow_id.clone(),
            run_id: Some(run_id.to_string()),
            origin_kind: "planned".to_string(),
            activation_status: "active".to_string(),
            name: workflow.name.clone(),
            definition: definition.clone(),
            version: workflow.version.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
        .map_err(|error| error.to_string())?;
        let workflow_version_id = uuid::Uuid::new_v4().to_string();
        db.save_workflow_version(&crate::core::models::WorkflowVersionRecord {
            id: workflow_version_id.clone(),
            workflow_id: workflow_id.clone(),
            run_id: Some(run_id.to_string()),
            instance_id: instance_id.to_string(),
            version: db
                .next_workflow_version_number(&workflow_id)
                .map_err(|error| error.to_string())?,
            definition_json: definition,
            validation_status: "valid".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
        .map_err(|error| error.to_string())?;
        db.update_orchestration_run_status(run_id, "running", Some(&workflow_id))
            .map_err(|error| error.to_string())?;
        db.set_setting("iflow_selected_run_id", run_id)
            .map_err(|error| error.to_string())?;
        let _ = db.insert_run_event(&crate::core::models::RunEventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            run_id: run_id.to_string(),
            event_type: "workflow_version_created".to_string(),
            actor_type: "system".to_string(),
            actor_id: None,
            task_id: None,
            payload: Some(format!(
                "Initial validated iFlow {} created for authoritative goal node dispatch.",
                workflow_id
            )),
            created_at: chrono::Utc::now().to_rfc3339(),
        });
        Ok(workflow_id)
    }

    #[cfg(any())]
    fn finish_initial_iflow_for_run(
        db: &Arc<dyn crate::core::traits::database::DatabasePort>,
        run_id: &str,
        run_status: &str,
    ) {
        let Ok(Some(version)) = db.get_latest_workflow_version_for_run(run_id) else {
            return;
        };
        let Ok(Some(mut execution)) = db.get_latest_workflow_execution_for_version(&version.id)
        else {
            return;
        };
        let Ok(mut state) = serde_json::from_str::<
            crate::application::iflow_engine::engine::WorkflowState,
        >(&execution.state_json) else {
            return;
        };
        match run_status {
            "completed" => {
                state.pending_agent_tasks.clear();
                state.completed_nodes.insert("goal".to_string());
                state.completed_nodes.insert("end".to_string());
                state.current_nodes = vec!["end".to_string()];
                state.status = crate::application::iflow_engine::engine::WorkflowStatus::Completed;
                execution.status = "completed".to_string();
            }
            "waiting_approval" => {
                state.status = crate::application::iflow_engine::engine::WorkflowStatus::Paused;
                execution.status = "waiting_approval".to_string();
            }
            "cancelled" => {
                state.status = crate::application::iflow_engine::engine::WorkflowStatus::Failed(
                    "Cancelled by user.".to_string(),
                );
                execution.status = "cancelled".to_string();
            }
            _ => {
                state.status = crate::application::iflow_engine::engine::WorkflowStatus::Failed(
                    "Chat execution did not complete.".to_string(),
                );
                execution.status = "failed".to_string();
            }
        }
        if let Ok(state_json) = serde_json::to_string(&state) {
            execution.state_json = state_json;
            execution.updated_at = chrono::Utc::now().to_rfc3339();
            let _ = db.save_workflow_state(&state);
            let _ = db.save_workflow_execution(&execution);
        }
    }

    pub(crate) fn handle_send_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let db = crate::AppState::global(cx).db.clone();
        if self.is_generating {
            if let Some(flag) = &self.generation_cancel_flag {
                flag.store(true, Ordering::SeqCst);
                window.push_notification(
                    (
                        gpui_component::notification::NotificationType::Info,
                        "Stopping generation...",
                    ),
                    cx,
                );
            }
            return;
        }

        let mut raw_text = self.chat_input_state.read(cx).text().to_string();
        raw_text = raw_text.trim().to_string();
        let slash_invocation = match parse_slash_invocation(&raw_text, self.selected_slash_command)
        {
            Ok(invocation) => invocation,
            Err(error) => {
                if matches!(error, SlashCommandParseError::EmptyCommand) {
                    self.is_slash_dropdown_open = true;
                    self.slash_command_selection_index = 0;
                    self.slash_command_active_query = Some(String::new());
                }
                window.push_notification(
                    (
                        gpui_component::notification::NotificationType::Info,
                        gpui::SharedString::from(error.message()),
                    ),
                    cx,
                );
                cx.notify();
                return;
            }
        };
        if let Some(invocation) = &slash_invocation {
            raw_text = invocation.normalized_input.clone();
            if invocation.command == SlashCommandId::Help {
                self.selected_slash_command = None;
                self.is_slash_dropdown_open = true;
                self.slash_command_selection_index = 0;
                self.slash_command_active_query = Some(String::new());
                window.push_notification(
                    (
                        gpui_component::notification::NotificationType::Info,
                        gpui::SharedString::from(slash_help_text()),
                    ),
                    cx,
                );
                cx.notify();
                return;
            }
            self.selected_slash_command = None;
            self.slash_command_selection_index = 0;
            self.slash_command_active_query = None;
        }
        if raw_text.is_empty() && self.attached_files.is_empty() {
            return;
        }
        self.is_slash_dropdown_open = false;
        self.slash_command_selection_index = 0;
        self.slash_command_active_query = None;
        cx.notify();

        let instance_id = if let Some(id) = &self.selected_instance_id {
            id.clone()
        } else {
            return; // Don't send if no instance is selected
        };

        let team_id = self
            .instances
            .iter()
            .find(|i| i.id == *instance_id)
            .map(|i| i.team_id.clone())
            .unwrap_or_default();
        let mode = crate::AppState::global(cx)
            .mode_manager
            .lock()
            .unwrap()
            .current_mode();

        let session_id = if let Some(session_id) = &self.selected_session_id {
            session_id.clone()
        } else {
            let agent_id = db
                .get_instance_agents(&instance_id)
                .ok()
                .and_then(|ids| ids.first().cloned());
            let Some(agent_id) = agent_id else {
                window.push_notification(
                    (gpui_component::notification::NotificationType::Error, "No agents available for this instance/team. Add agents to the team (Manage Team Members) or assign agents to the instance."),
                    cx,
                );
                return;
            };
            let Ok(session_id) = db.create_session_for_instance(&instance_id, &agent_id) else {
                return;
            };
            self.selected_session_id = Some(session_id.clone());
            self.instance_active_session
                .insert(instance_id.clone(), session_id.clone());
            self.sessions_for_instance = db
                .list_sessions_for_instance(&instance_id)
                .unwrap_or_default();
            session_id
        };

        let attached_files_for_ai = self.attached_files.clone();
        let mut text = if let Some(invocation) = &slash_invocation {
            let ts = Utc::now().format("%Y%m%d_%H%M%S").to_string();
            build_slash_prompt(invocation, &ts)
                .unwrap_or_else(|| invocation.normalized_input.clone())
        } else if raw_text.is_empty() {
            "Please analyze the attached file(s).".to_string()
        } else {
            raw_text.clone()
        };
        if !attached_files_for_ai.is_empty() {
            text.push_str("\n\nAttached files:\n");
            for file in &attached_files_for_ai {
                text.push_str(&format!("- {}\n", file));
            }
        }

        let run_id = uuid::Uuid::new_v4().to_string();
        let run_created_at = chrono::Utc::now().to_rfc3339();
        let current_actor_id = crate::AppState::global(cx).current_actor_id.clone();
        let run_goal = if raw_text.trim().is_empty() {
            "Analyze attached files".to_string()
        } else {
            raw_text.clone()
        };
        let run = crate::core::models::OrchestrationRunRecord {
            id: run_id.clone(),
            session_id: session_id.clone(),
            instance_id: instance_id.clone(),
            initiated_by: Some(current_actor_id.clone()),
            goal: run_goal,
            mode: mode.storage_value().to_string(),
            status: "running".to_string(),
            workflow_id: None,
            created_at: run_created_at.clone(),
            updated_at: run_created_at.clone(),
        };
        if let Err(error) = db.create_orchestration_run(&run) {
            window.push_notification(
                (
                    gpui_component::notification::NotificationType::Error,
                    gpui::SharedString::from(format!("Unable to start traceable run: {}", error)),
                ),
                cx,
            );
            return;
        }
        let _ = db.insert_run_event(&crate::core::models::RunEventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            run_id: run_id.clone(),
            event_type: "run_created".to_string(),
            actor_type: "user".to_string(),
            actor_id: Some(current_actor_id),
            task_id: None,
            payload: Some(format!("Session: {}", session_id)),
            created_at: run_created_at,
        });
        let mut initial_workflow_id = None;
        if slash_invocation
            .as_ref()
            .map_or(true, |invocation| invocation.command != SlashCommandId::Run)
        {
            if let Some(agent_id) = db
                .get_instance_agents(&instance_id)
                .ok()
                .and_then(|ids| ids.first().cloned())
            {
                if let Ok(workflow_id) = Self::persist_initial_iflow_for_run(
                    &db,
                    &run_id,
                    &team_id,
                    &instance_id,
                    &agent_id,
                    &text,
                ) {
                    initial_workflow_id = Some(workflow_id);
                    self.available_iflow_run_id = Some(run_id.clone());
                    let selected_iflow = crate::AppState::global(cx).selected_iflow_run_id.clone();
                    let run_for_selection = run_id.clone();
                    selected_iflow.update(cx, move |selected, cx| {
                        *selected = Some(run_for_selection);
                        cx.notify();
                    });
                }
            }
        }

        {
            let history = self.chat_histories.entry(session_id.clone()).or_default();
            history.push(crate::providers::ChatMessage {
                role: "user".into(),
                content: text.clone().into(),
                parts: vec![],
                agent_name: None,
                thought_duration_secs: None,
            });
        }
        self.rebuild_chat_display(&session_id);
        let display_len = self
            .chat_display_rows
            .get(&session_id)
            .map(|v| v.len())
            .unwrap_or(0);
        self.chat_list_state =
            gpui::ListState::new(display_len, gpui::ListAlignment::Bottom, px(200.));
        let history_snapshot = self
            .chat_histories
            .get(&session_id)
            .cloned()
            .unwrap_or_default();

        // Sync user message to Office view
        {
            let first_agent_id = db
                .get_instance_agents(&instance_id)
                .ok()
                .and_then(|ids| ids.into_iter().next())
                .unwrap_or_default();
            self.push_office_chat_message(&first_agent_id, &text, true, "You", cx);
        }

        let user_msg = crate::teambus::routing::TeamMessage::new_broadcast(
            instance_id.clone(),
            "user".to_string(),
            text.clone(),
        );
        let _ = db.insert_team_message(&user_msg);
        let team_bus = self.team_bus.clone();
        let user_msg_clone = user_msg.clone();
        cx.spawn(async move |_, _| {
            let _ = team_bus.route_message(user_msg_clone).await;
        })
        .detach();

        if let Ok(agent_ids) = db.get_instance_agents(&instance_id) {
            if let Some(agent_id) = agent_ids.first() {
                let _ = db.ensure_session(&session_id, agent_id, Some(&instance_id));
                let _ = db.append_conversation_turn(&session_id, "user", &text, None);
                let _ = db.touch_session(&session_id);
            }
        }

        // Audit log: record user chat message
        let audit_event = crate::infrastructure::security::audit::AuditEvent {
            timestamp: chrono::Utc::now(),
            action: "chat_message".to_string(),
            user_id: Some("user".to_string()),
            resource: instance_id.clone(),
            details: format!(
                "User message ({} chars) in session {}",
                text.len(),
                session_id
            ),
        };
        let _ = db.insert_audit_log(&audit_event);

        if let Some(target_instance_id) = self.cross_team_target_instance_id.clone() {
            if target_instance_id != instance_id {
                let correlation_id = uuid::Uuid::new_v4().to_string();
                let context_refs_json = serde_json::json!({
                    "session_id": &session_id,
                    "run_id": &run_id
                })
                .to_string();
                let handoff_result =
                    crate::application::orchestration::collaboration::CollaborationService::new(
                        db.clone(),
                    )
                    .create_handoff(
                        crate::application::orchestration::collaboration::HandoffInput {
                            run_id: Some(&run_id),
                            correlation_id: Some(&correlation_id),
                            from_instance_id: &instance_id,
                            to_instance_id: &target_instance_id,
                            from_agent_id: None,
                            objective: &text,
                            acceptance_json: "[]",
                            constraints_json: "{}",
                            context_refs_json: &context_refs_json,
                            priority: "medium",
                            risk_level: "medium",
                        },
                    );
                match handoff_result {
                    Ok((case_id, handoff_id, persisted_correlation_id)) => {
                        let payload = serde_json::json!({
                            "handoff_type": "message",
                            "correlation_id": persisted_correlation_id,
                            "case_id": case_id,
                            "handoff_id": handoff_id,
                            "from_team": &instance_id,
                            "reply_to_team": &instance_id,
                            "briefing_package": &text
                        })
                        .to_string();
                        let content = format!("[CROSS_TEAM_HANDOFF] {}", payload);

                        let mut cross_msg = crate::teambus::routing::TeamMessage::new_broadcast(
                            target_instance_id.clone(),
                            "cross-team".to_string(),
                            content.clone(),
                        );
                        cross_msg.metadata = Some(payload);
                        let _ = db.insert_team_message(&cross_msg);
                        let team_bus = self.team_bus.clone();
                        let cross_msg_clone = cross_msg.clone();
                        cx.spawn(async move |_, _| {
                            let _ = team_bus.route_message(cross_msg_clone).await;
                        })
                        .detach();

                        let target_agent_id = db
                            .get_instance_agents(&target_instance_id)
                            .ok()
                            .and_then(|ids| ids.first().cloned());
                        if let Some(target_agent_id) = target_agent_id {
                            let mut session = db
                                .get_latest_session_for_instance(&target_instance_id)
                                .ok()
                                .flatten();
                            if session.is_none() {
                                let _ = db.create_session_for_instance(
                                    &target_instance_id,
                                    &target_agent_id,
                                );
                                session = db
                                    .get_latest_session_for_instance(&target_instance_id)
                                    .ok()
                                    .flatten();
                            }
                            if let Some(session) = session {
                                let meta =
                                    serde_json::json!({"agent_name":"Cross-team"}).to_string();
                                let _ = db.ensure_session(
                                    &session.id,
                                    &target_agent_id,
                                    Some(&target_instance_id),
                                );
                                let _ = db.append_conversation_turn(
                                    &session.id,
                                    "assistant",
                                    &content,
                                    Some(&meta),
                                );
                                let _ = db.touch_session(&session.id);
                            }
                        }
                    }
                    Err(error) => {
                        let _ = db.insert_run_event(&crate::core::models::RunEventRecord {
                            id: uuid::Uuid::new_v4().to_string(),
                            run_id: run_id.clone(),
                            event_type: "governed_handoff_persist_failed".to_string(),
                            actor_type: "system".to_string(),
                            actor_id: None,
                            task_id: None,
                            payload: Some(error.to_string()),
                            created_at: chrono::Utc::now().to_rfc3339(),
                        });
                        window.push_notification(
                            (
                                gpui_component::notification::NotificationType::Error,
                                gpui::SharedString::from(format!(
                                    "Unable to send governed handoff: {}",
                                    error
                                )),
                            ),
                            cx,
                        );
                    }
                }
            }
        }

        let is_run_command = slash_invocation.as_ref().map_or(false, |invocation| {
            invocation.command == SlashCommandId::Run
        });
        let mode_clone = mode;
        let db_clone = db.clone();
        let _text_clone = text.clone();
        let team_bus_clone = self.team_bus.clone();
        let instance_id_clone = instance_id.clone();
        let team_id_clone = team_id.clone();
        let session_id_clone = session_id.clone();
        let run_id_clone = run_id.clone();
        let _history_clone = history_snapshot.clone();
        let view = cx.entity().clone();
        let workspace_dir_clone = self.workspace_path.clone();
        self.attached_files.clear();

        if !is_run_command {
            self.chat_input_state.update(cx, |state, cx| {
                state.set_value("", window, cx);
            });
            let dispatch_result = initial_workflow_id
                .as_deref()
                .ok_or_else(|| "Unable to create the authoritative iFlow for this run.".to_string())
                .and_then(|workflow_id| {
                    let app = crate::AppState::global(cx);
                    crate::application::iflow_engine::automation::IFlowAutomation::dispatch_persisted_workflow(
                        db.clone(),
                        self.team_bus.clone(),
                        app.tokio_runtime.clone(),
                        workflow_id,
                    )
                    .map(|_| ())
                });
            if let Err(error) = dispatch_result {
                let _ = db.update_orchestration_run_status(&run_id, "failed", None);
                let _ = db.insert_run_event(&crate::core::models::RunEventRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    run_id: run_id.clone(),
                    event_type: "workflow_dispatch_failed".to_string(),
                    actor_type: "system".to_string(),
                    actor_id: None,
                    task_id: None,
                    payload: Some(error.clone()),
                    created_at: chrono::Utc::now().to_rfc3339(),
                });
                window.push_notification(
                    (
                        gpui_component::notification::NotificationType::Error,
                        gpui::SharedString::from(error),
                    ),
                    cx,
                );
            }
            cx.notify();
            return;
        }

        if is_run_command {
            cx.spawn(async move |_, cx| {
                let _is_run_command = true;
                let _mode_clone = mode_clone;
                let _text_clone = "".to_string(); // Not used

                let all_agent_ids = db_clone.get_instance_agents(&instance_id_clone).unwrap_or_default();
                if all_agent_ids.is_empty() { return; }

                for agent_id in all_agent_ids {
                    let agent_id_clone = agent_id.clone();
                    let db_clone_agent = db_clone.clone();
                    let team_id_clone_agent = team_id_clone.clone();
                    let instance_id_clone_agent = instance_id_clone.clone();
                    let team_bus_clone_agent = team_bus_clone.clone();
                    let view_agent = view.clone();
                    let session_id_clone_agent = session_id_clone.clone();
                    let run_id_clone_agent = run_id_clone.clone();
                    let workspace_dir_agent = workspace_dir_clone.clone();

                    cx.spawn(async move |cx| {
                        loop {
                            let Ok(Some(agent)) = db_clone_agent.get_agent(&agent_id_clone) else { break; };
                            if agent.status.to_lowercase() == "offline" {
                                break;
                            }
                            let provider_config = db_clone_agent
                                .get_provider_by_name(&agent.provider)
                                .ok()
                                .flatten()
                                .or_else(|| {
                                    db_clone_agent.list_providers().ok().and_then(|providers| {
                                        providers
                                            .into_iter()
                                            .find(|p| provider_kind(p) == agent.provider.as_str())
                                    })
                                });
                            let Some(provider_config) = provider_config else { break; };

                            let tasks = db_clone_agent.list_tasks_for_instance(&instance_id_clone_agent).unwrap_or_default();
                            let agent_route_key = agent
                                .routing_role()
                                .trim()
                                .to_lowercase()
                                .chars()
                                .filter(|ch| ch.is_ascii_alphanumeric())
                                .collect::<String>();
                            let task_matches_agent = |task: &crate::tasks::shared_task_list::Task| {
                                if task.assignee_id.as_ref() == Some(&agent_id_clone) {
                                    return true;
                                }
                                if task.assignee_id.is_some() {
                                    return false;
                                }
                                let Some(payload) = task.payload.as_deref() else {
                                    return false;
                                };
                                let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
                                    return false;
                                };
                                let route = value
                                    .get("role")
                                    .and_then(|role| role.as_str())
                                    .or_else(|| value.get("name").and_then(|name| name.as_str()))
                                    .unwrap_or("")
                                    .trim()
                                    .to_lowercase()
                                    .chars()
                                    .filter(|ch| ch.is_ascii_alphanumeric())
                                    .collect::<String>();
                                !route.is_empty() && route == agent_route_key
                            };
                            let task_capability_violation = |task: &crate::tasks::shared_task_list::Task| -> Option<String> {
                                let payload = task.payload.as_deref()?;
                                let value = serde_json::from_str::<serde_json::Value>(payload).ok()?;
                                let field = |key: &str| {
                                    value
                                        .get(key)
                                        .and_then(|field| field.as_str())
                                        .map(str::trim)
                                        .filter(|field| !field.is_empty())
                                        .unwrap_or("")
                                        .to_string()
                                };
                                let mut title = field("title");
                                if title.is_empty() {
                                    title = field("name");
                                }
                                crate::application::orchestration::role_policy::validate_agent_task_assignment(
                                    &agent,
                                    &field("task_type"),
                                    &title,
                                    &field("description"),
                                )
                                .err()
                            };

                            let mut next_task = None;
                            for task in &tasks {
                                if task.status == "pending" && task_matches_agent(task) {
                                    if db_clone_agent.is_task_unblocked(&task.id).unwrap_or(false) {
                                        if let Some(reason) = task_capability_violation(task) {
                                            let status_text = format!("[Task Routing Error] {}:\n{}", task.id, reason);
                                            let _ = db_clone_agent.mark_task_failed(&task.id);
                                            let metadata = chat_message_metadata(&agent.name, None);
                                            let mut msg = crate::teambus::routing::TeamMessage::new_broadcast(
                                                instance_id_clone_agent.clone(),
                                                "assistant".to_string(),
                                                status_text.clone(),
                                            );
                                            msg.metadata = Some(metadata.clone());
                                            let _ = db_clone_agent.insert_team_message(&msg);
                                            let _ = team_bus_clone_agent.route_message(msg).await;
                                            let _ = db_clone_agent.append_conversation_turn(
                                                &session_id_clone_agent,
                                                "assistant",
                                                &status_text,
                                                Some(&metadata),
                                            );
                                            continue;
                                        }
                                        next_task = Some(task.clone());
                                        break;
                                    }
                                }
                            }

                            let Some(task) = next_task else {
                                // If there are pending tasks but dependencies aren't met, wait and retry.
                                let has_pending = tasks.iter().any(|t| t.status == "pending" && task_matches_agent(t));
                                if has_pending {
                                    cx.background_executor().timer(std::time::Duration::from_secs(2)).await;
                                    continue;
                                } else {
                                    break;
                                }
                            };

                            // Claim task (sets to in_progress)
                            let claimed = db_clone_agent
                                .claim_task_for_instance(&task.id, &agent_id_clone, &instance_id_clone_agent)
                                .unwrap_or(false);
                            if !claimed {
                                continue;
                            }

                            let mut task_prompt = Vec::new();
                            let chat_service = crate::application::services::chat_service::ChatService::new(db_clone_agent.clone(), team_bus_clone_agent.clone());
                            if let Some(sys_prompt) = chat_service.build_dynamic_system_prompt(&team_id_clone_agent, &instance_id_clone_agent, &agent_id_clone) {
                                task_prompt.push(crate::core::models::ChatMessage { role: "system".into(), content: gpui::SharedString::from(sys_prompt), agent_name: None, thought_duration_secs: None , parts: vec![] });
                            }

                            // Instruct the LLM to output files if needed
                            let instructions = if let Some(ref ws) = workspace_dir_agent {
                                format!("Execute the following task. You are working in the directory: {}. To generate or modify files, call write_file or edit_file with a relative path inside this workspace. Do not use run_cli to create directories/files when write_file can create parent directories automatically. Use run_cli only for actual build/test/diagnostic commands. Sensitive tools may pause for governance approval. Do not express file operations as markdown. Task:\n", ws)
                            } else {
                                "Execute the following task. A configured workspace is required for file changes, and file operations must use write_file or edit_file tools. Do not use run_cli to create directories/files. Sensitive tools may pause for governance approval. Do not express file operations as markdown. Task:\n".to_string()
                            };

                            let task_text = task.payload.as_deref().map(|payload| {
                                serde_json::from_str::<serde_json::Value>(payload)
                                    .ok()
                                    .map(|value| {
                                        let field = |key: &str| {
                                            value
                                                .get(key)
                                                .and_then(|field| field.as_str())
                                                .map(str::trim)
                                                .filter(|field| !field.is_empty())
                                                .unwrap_or("")
                                                .to_string()
                                        };
                                        let description = field("description");
                                        let mut title = field("title");
                                        if title.is_empty() {
                                            title = field("name");
                                        }
                                        let role = field("role");
                                        let title_key = title
                                            .trim()
                                            .to_lowercase()
                                            .chars()
                                            .filter(|ch| ch.is_ascii_alphanumeric())
                                            .collect::<String>();
                                        let role_key = role
                                            .trim()
                                            .to_lowercase()
                                            .chars()
                                            .filter(|ch| ch.is_ascii_alphanumeric())
                                            .collect::<String>();
                                        if !title_key.is_empty()
                                            && (title_key == role_key
                                                || matches!(
                                                    title_key.as_str(),
                                                    "coordinator"
                                                        | "pm"
                                                        | "ba"
                                                        | "dev"
                                                        | "developer"
                                                        | "engineer"
                                                ))
                                        {
                                            if let Some(first_line) = description
                                                .lines()
                                                .map(str::trim)
                                                .find(|line| !line.is_empty())
                                            {
                                                title = first_line
                                                    .trim_start_matches(|ch: char| {
                                                        ch.is_ascii_digit()
                                                            || ch == '.'
                                                            || ch == ')'
                                                            || ch == '-'
                                                    })
                                                    .trim()
                                                    .chars()
                                                    .take(96)
                                                    .collect();
                                            }
                                        }
                                        let mut text = String::new();
                                        if !title.is_empty() {
                                            text.push_str("Title: ");
                                            text.push_str(&title);
                                            text.push('\n');
                                        }
                                        if !role.is_empty() {
                                            text.push_str("Assigned role: ");
                                            text.push_str(&role);
                                            text.push('\n');
                                        }
                                        if !description.is_empty() {
                                            text.push_str("Description:\n");
                                            text.push_str(&description);
                                        }
                                        if text.trim().is_empty() {
                                            payload.to_string()
                                        } else {
                                            text
                                        }
                                    })
                                    .unwrap_or_else(|| payload.to_string())
                            }).unwrap_or_else(|| task.id.clone());

                            task_prompt.push(crate::providers::ChatMessage { role: "user".into(), content: format!("{}{}", instructions, task_text).into(), agent_name: None, thought_duration_secs: None , parts: vec![] });

                            let agent_name_str = agent.name.clone();
                            let metadata = chat_message_metadata(&agent_name_str, None);
                            let mut msg = crate::teambus::routing::TeamMessage::new_broadcast(
                                instance_id_clone_agent.clone(),
                                "assistant".to_string(),
                                String::new(),
                            );
                            msg.metadata = Some(metadata.clone());
                            msg.delivery_status = "typing".to_string();
                            let msg_id = msg.id.clone();
                            let _ = db_clone_agent.insert_team_message(&msg);
                            let _ = team_bus_clone_agent.route_message(msg.clone()).await;

                            let mut msg_idx = None;
                            let _ = cx.update(|cx| view_agent.update(cx, |this: &mut Self, cx| {
                                let session_id = this
                                    .instance_active_session
                                    .get(&instance_id_clone_agent)
                                    .cloned()
                                    .or_else(|| this.selected_session_id.clone());
                                if let Some(session_id) = session_id {
                                    let history = this.chat_histories.entry(session_id.clone()).or_default();
                                    history.push(crate::providers::ChatMessage {
                                        role: "assistant".into(),
                                        content: "".into(),
                                        parts: vec![],
                                        agent_name: Some(agent_name_str.clone().into()),
                                        thought_duration_secs: None,
                                    });
                                    msg_idx = history.len().checked_sub(1);
                                    this.rebuild_chat_display(&session_id);
                                    this.refresh_pending_chat_action(cx);
                                    if this.selected_session_id.as_deref() == Some(session_id.as_str()) {
                                        let display_len = this
                                            .chat_display_rows
                                            .get(&session_id)
                                            .map(|v| v.len())
                                            .unwrap_or(0);
                                        this.chat_list_state = gpui::ListState::new(
                                            display_len,
                                            gpui::ListAlignment::Bottom,
                                            gpui::px(200.),
                                        );
                                    }
                                }
                                cx.notify();
                            }));

                            let (stream_tx, mut stream_rx) =
                                tokio::sync::watch::channel(String::new());
                            let view_stream = view_agent.clone();
                            let db_stream = db_clone_agent.clone();
                            let msg_id_stream = msg_id.clone();
                            let session_id_stream = session_id_clone_agent.clone();
                            let msg_idx_stream = msg_idx;
                            cx.spawn(async move |cx| {
                                while stream_rx.changed().await.is_ok() {
                                    let partial = stream_rx.borrow_and_update().clone();
                                    if partial.is_empty() {
                                        continue;
                                    }
                                    let _ = db_stream.update_team_message_content(&msg_id_stream, &partial);
                                    let Some(msg_idx) = msg_idx_stream else {
                                        continue;
                                    };
                                    let _ = cx.update(|cx| {
                                        view_stream.update(cx, |this: &mut Self, cx| {
                                            this.update_chat_message_content(
                                                &session_id_stream,
                                                msg_idx,
                                                partial.clone(),
                                                None,
                                                cx,
                                            );
                                        });
                                    });
                                }
                            }).detach();

                            let result = if let Some(adapter) = build_provider_adapter(&provider_config) {
                                let mcp_registry = std::sync::Arc::new(
                                    crate::infrastructure::mcp::registry::McpToolRegistry::new(
                                        db_clone_agent.clone(),
                                    ),
                                );
                                let executor =
                                    crate::application::orchestration::executor::AgentExecutor::new(
                                        adapter,
                                        mcp_registry,
                                        db_clone_agent.clone(),
                                        team_bus_clone_agent.clone(),
                                        instance_id_clone_agent.clone(),
                                        agent_id_clone.clone(),
                                        Some(session_id_clone_agent.clone()),
                                        task.run_id
                                            .clone()
                                            .or_else(|| Some(run_id_clone_agent.clone())),
                                        None,
                                        Some(std::sync::Arc::new(move |partial| {
                                            stream_tx.send_replace(partial);
                                        })),
                                    );
                                let response_started_at = std::time::Instant::now();
                                executor.execute_task(task_prompt).await
                                    .map(|text| (text, response_started_at.elapsed().as_secs_f64()))
                                    .map_err(|error| (error, response_started_at.elapsed().as_secs_f64()))
                            } else {
                                Err((
                                    anyhow::anyhow!(
                                    "Provider adapter is not supported for /run: {}",
                                    provider_config.provider_name
                                    ),
                                    0.0,
                                ))
                            };

                            let thought_duration_secs = match &result {
                                Ok((_, seconds)) | Err((_, seconds)) => *seconds,
                            };
                            let (status_text, status) = match result {
                                Ok((text, _)) if text.starts_with("Approval required before executing") => (
                                    text,
                                    "waiting_approval",
                                ),
                                Ok((text, _)) => {
                                    let chat_service = crate::application::services::chat_service::ChatService::new(db_clone_agent.clone(), team_bus_clone_agent.clone());
                                    let (files_written, _) = chat_service.parse_generated_response(&text, workspace_dir_agent.as_ref());

                                    let mut final_text = text;
                                    if !files_written.is_empty() {
                                        final_text.push_str("\n\n**Files Generated/Modified:**\n");
                                        for f in files_written {
                                            // Strip absolute path
                                            let display_path = if let Some(ws) = workspace_dir_agent.as_ref() {
                                                f.replace(ws, "")
                                            } else {
                                                std::path::Path::new(&f).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or(f)
                                            };
                                            let display_path = display_path.trim_start_matches('/').trim_start_matches('\\');
                                            final_text.push_str(&format!("- `{}`\n", display_path));
                                        }
                                    }

                                    (final_text, "completed")
                                },
                                Err((e, _)) => (format!("Task failed: {}", e), "failed"),
                            };

                            let _ = match status {
                                "completed" => db_clone_agent.mark_task_completed(&task.id),
                                "waiting_approval" => db_clone_agent.mark_task_waiting_approval(&task.id),
                                _ => db_clone_agent.mark_task_failed(&task.id),
                            };

                            let final_delivery_status = if status == "completed" {
                                "delivered"
                            } else {
                                status
                            };
                            let _ = db_clone_agent.update_team_message_content(&msg_id, &status_text);
                            let _ = db_clone_agent
                                .update_team_message_delivery_status(&msg_id, final_delivery_status);
                            let mut final_msg = msg.clone();
                            final_msg.content = status_text.clone();
                            final_msg.delivery_status = final_delivery_status.to_string();
                            final_msg.created_at = chrono::Utc::now().to_rfc3339();
                            let final_metadata =
                                chat_message_metadata(&agent_name_str, Some(thought_duration_secs));
                            final_msg.metadata = Some(final_metadata.clone());
                            let _ = team_bus_clone_agent.route_message(final_msg).await;

                            // Save to database so it persists across reloads!
                            let _ = db_clone_agent.ensure_session(&session_id_clone_agent, &agent_id_clone, Some(&instance_id_clone_agent));
                            let _ = db_clone_agent.append_conversation_turn(
                                &session_id_clone_agent,
                                "assistant",
                                &status_text,
                                Some(&final_metadata),
                            );
                            let _ = db_clone_agent.touch_session(&session_id_clone_agent);

                            let _ = cx.update(|cx| view_agent.update(cx, |this: &mut Self, cx| {
                                let session_id = this
                                    .instance_active_session
                                    .get(&instance_id_clone_agent)
                                    .cloned()
                                    .or_else(|| this.selected_session_id.clone());
                                if let Some(session_id) = session_id {
                                    if let Some(msg_idx) = msg_idx {
                                        this.update_chat_message_content(
                                            &session_id,
                                            msg_idx,
                                            status_text.clone(),
                                            Some(thought_duration_secs),
                                            cx,
                                        );
                                    }
                                    this.rebuild_chat_display(&session_id);
                                    this.refresh_pending_chat_action(cx);
                                    if this.selected_session_id.as_deref() == Some(session_id.as_str()) {
                                        let display_len = this
                                            .chat_display_rows
                                            .get(&session_id)
                                            .map(|v| v.len())
                                            .unwrap_or(0);
                                        this.chat_list_state = gpui::ListState::new(
                                            display_len,
                                            gpui::ListAlignment::Bottom,
                                            gpui::px(200.),
                                        );
                                    }
                                }
                                cx.notify();
                            }));
                        }
                    }).detach();
                }
            }).detach();
            let _ = db.update_orchestration_run_status(&run_id, "dispatched", None);
            let _ = db.insert_run_event(&crate::core::models::RunEventRecord {
                id: uuid::Uuid::new_v4().to_string(),
                run_id: run_id.clone(),
                event_type: "task_dispatch_started".to_string(),
                actor_type: "system".to_string(),
                actor_id: None,
                task_id: None,
                payload: Some("Workers started for queued tasks".to_string()),
                created_at: chrono::Utc::now().to_rfc3339(),
            });
            self.chat_input_state.update(cx, |state, cx| {
                state.set_value("", window, cx);
            });
            cx.notify();
            return;
        }

        // Retained temporarily for source migration history; new chat requests return above
        // after dispatching their persisted iFlow and cannot enter this legacy direct path.
        #[cfg(any())]
        {
            let cancel_flag = Arc::new(AtomicBool::new(false));
            self.is_generating = true;
            self.generation_cancel_flag = Some(cancel_flag.clone());
            self.chat_list_state =
                gpui::ListState::new(display_len, gpui::ListAlignment::Bottom, px(200.));
            self.chat_input_state.update(cx, |state, cx| {
                state.set_value("", window, cx);
            });
            cx.notify();

            // Trigger AI response asynchronously
            let history_clone = history_snapshot.clone();
            let view = cx.entity().clone();
            let db = db.clone();
            let team_id_clone = team_id.clone();
            let query_text = text.clone();
            let instance_id_for_ai = instance_id.clone();
            let session_id_for_ai = session_id.clone();
            let run_id_for_ai = run_id.clone();

            let db_clone = db.clone();
            let team_bus_for_ai = self.team_bus.clone();
            let team_bus_clone = self.team_bus.clone();
            let workspace_dir_for_ai = workspace_dir_clone.clone();
            let attached_files_for_context = attached_files_for_ai.clone();
            let cancel_flag_for_ai = cancel_flag.clone();
            cx.spawn(async move |_, cx| {
            use crate::providers::BaseProviderAdapter;

            let mut use_mock = true;
            let mut produced_response = false;

            if cancel_flag_for_ai.load(Ordering::SeqCst) {
                let _ = db.update_orchestration_run_status(&run_id_for_ai, "cancelled", None);
                let _ = db.insert_run_event(&crate::core::models::RunEventRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    run_id: run_id_for_ai.clone(),
                    event_type: "run_cancelled".to_string(),
                    actor_type: "system".to_string(),
                    actor_id: None,
                    task_id: None,
                    payload: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                });
                let _ = cx.update(|cx| {
                    view.update(cx, |this: &mut Self, cx| {
                        this.is_generating = false;
                        this.generation_cancel_flag = None;
                        cx.notify();
                    });
                });
                return;
            }

            if let Ok(agent_ids) = db.get_instance_agents(&instance_id_for_ai) {
                let agent_ids: Vec<String> = agent_ids;
                let mut current_history = history_clone.clone();
                let auto_context_result =
                    crate::application::file_intelligence::build_chat_context_with_sources(
                    &query_text,
                    &attached_files_for_context,
                    workspace_dir_for_ai.as_deref(),
                    crate::application::file_intelligence::AnalyzeOptions {
                        max_text_chars: 18_000,
                        ..crate::application::file_intelligence::AnalyzeOptions::default()
                    },
                )
                .await;
                if !auto_context_result.markdown.trim().is_empty() {
                    if let Some(last_user) = current_history
                        .iter_mut()
                        .rev()
                        .find(|msg| msg.role == "user")
                    {
                        last_user.content =
                            format!("{}{}", last_user.content, auto_context_result.markdown).into();
                    }
                }
                if !auto_context_result.url_analyses.is_empty() {
                    let results = auto_context_result
                        .url_analyses
                        .into_iter()
                        .map(crate::application::research::web::web_search_result_from_analysis)
                        .collect::<Vec<_>>();
                    let notebook = crate::application::research::web::build_research_notebook(
                        &query_text,
                        &results,
                    );
                    let _ =
                        crate::application::research::web::save_research_notebook_with_context(
                            db.clone(),
                            &query_text,
                            &notebook,
                            crate::application::research::web::ResearchNotebookSaveContext {
                                source_kind: Some("web_research".to_string()),
                                source_uri_normalized: None,
                                origin_run_id: Some(run_id_for_ai.clone()),
                                origin_session_id: Some(session_id_for_ai.clone()),
                                origin_instance_id: Some(instance_id_for_ai.clone()),
                                origin_agent_id: None,
                            },
                        )
                        .await;
                }

                let debate_steps: Vec<(String, &'static str)> = if agent_ids.len() > 1 {
                    let mut steps = vec![
                        (agent_ids[0].clone(), "PROPOSER"),
                        (agent_ids[1].clone(), "CRITIC"),
                        (agent_ids[0].clone(), "RESOLVER"),
                    ];
                    if agent_ids.len() > 2 {
                        steps.push((agent_ids[2].clone(), "JUDGE"));
                    }
                    steps
                } else {
                    agent_ids
                        .first()
                        .map(|id| vec![(id.clone(), "SOLO")])
                        .unwrap_or_default()
                };

                for (step_ix, (agent_id, debate_role)) in debate_steps.iter().enumerate() {
                    if cancel_flag_for_ai.load(Ordering::SeqCst) {
                        break;
                    }
                    if !agent_id.is_empty() {
                        let agent_id = agent_id.clone();
                        if let Ok(Some(agent)) = db.get_agent(&agent_id) {
                            if agent.status.to_lowercase() == "offline" {
                                continue;
                            }
                            let provider_config = db
                                .get_provider_by_name(&agent.provider)
                                .ok()
                                .flatten()
                                .or_else(|| {
                                    db.list_providers().ok().and_then(|providers| {
                                        providers
                                            .into_iter()
                                            .find(|p| provider_kind(p) == agent.provider.as_str())
                                    })
                                });
                            let Some(provider_config) = provider_config else {
                                continue;
                            };
                            use_mock = false;

                            let chat_service = crate::application::services::chat_service::ChatService::new(db.clone(), team_bus_for_ai.clone());
                            let mut full_history = current_history.clone();

                            if let Some(mut sys) = chat_service.build_dynamic_system_prompt(&team_id_clone, &instance_id_for_ai, &agent_id) {
                                if *debate_role != "SOLO" {
                                    sys.push_str("\n\nDEBATE PROTOCOL\nYou are collaborating with other agents in the same team. You MUST read previous agent responses in the chat history.\n");
                                    match *debate_role {
                                        "PROPOSER" => {
                                            sys.push_str("ROLE: PROPOSER\nProvide an initial solution/plan. Be concrete and actionable.\n");
                                        }
                                        "CRITIC" => {
                                            sys.push_str("ROLE: CRITIC\nCritique the proposer response. Identify flaws, missing steps, security risks, and mismatches with requirements. Provide a numbered issue list.\n");
                                        }
                                        "RESOLVER" => {
                                            sys.push_str("ROLE: RESOLVER\nRevise the proposal to address ALL critique issues. Output the revised plan. If it is final and agreed, include [CONSENSUS_REACHED].\n");
                                        }
                                        "JUDGE" => {
                                            sys.push_str("ROLE: JUDGE\nDecide if the latest plan is ready to execute. If acceptable, include [CONSENSUS_REACHED]. If not, list blocking issues.\n");
                                        }
                                        _ => {}
                                    }
                                }
                                full_history.insert(0, crate::providers::ChatMessage { role: gpui::SharedString::from("system"), content: gpui::SharedString::from(sys), agent_name: None, thought_duration_secs: None , parts: vec![] });
                            }

                            let mut round_result: Option<String> = None;
                            if let Some(adapter) = build_provider_adapter(&provider_config) {
                                let agent_name_str = agent.name.clone();
                                let metadata = chat_message_metadata(&agent_name_str, None);
                                let mut office_msg = crate::teambus::routing::TeamMessage::new_broadcast(
                                    instance_id_for_ai.clone(),
                                    "assistant".to_string(),
                                    format!("[{}]: ", agent.name),
                                );
                                office_msg.metadata = Some(metadata.clone());
                                office_msg.delivery_status = "typing".to_string();
                                let office_msg_id = office_msg.id.clone();
                                let _ = db_clone.insert_team_message(&office_msg);

                                let mut msg_idx = 0;
                                let _ = cx.update(|cx| {
                                    view.update(cx, |this: &mut Self, cx| {
                                        {
                                            let history = this
                                                .chat_histories
                                                .entry(session_id_for_ai.clone())
                                                .or_default();
                                            history.push(crate::providers::ChatMessage {
                                                role: "assistant".into(),
                                                content: "".into(),
                                                parts: vec![],
                                                agent_name: Some(agent.name.clone().into()),
                                                thought_duration_secs: None,
                                            });
                                            msg_idx = history.len() - 1;
                                        }
                                        this.rebuild_chat_display(&session_id_for_ai);
                                        this.refresh_pending_chat_action(cx);
                                        let display_len = this
                                            .chat_display_rows
                                            .get(&session_id_for_ai)
                                            .map(|v| v.len())
                                            .unwrap_or(0);
                                        this.chat_list_state = gpui::ListState::new(
                                            display_len,
                                            gpui::ListAlignment::Bottom,
                                            px(200.),
                                        );
                                        cx.notify();
                                    });
                                });

                                let (stream_tx, mut stream_rx) =
                                    tokio::sync::watch::channel(String::new());
                                let view_stream = view.clone();
                                let db_stream = db_clone.clone();
                                let office_msg_id_stream = office_msg_id.clone();
                                let session_id_stream = session_id_for_ai.clone();
                                let msg_idx_stream = msg_idx;
                                cx.spawn(async move |cx| {
                                    while stream_rx.changed().await.is_ok() {
                                        let partial = stream_rx.borrow_and_update().clone();
                                        if partial.is_empty() {
                                            continue;
                                        }
                                        let _ = db_stream.update_team_message_content(
                                            &office_msg_id_stream,
                                            &partial,
                                        );
                                        let _ = cx
                                            .update(|cx| {
                                                view_stream.update(cx, |this: &mut Self, cx| {
                                                    this.update_chat_message_content(
                                                        &session_id_stream,
                                                        msg_idx_stream,
                                                        partial.clone(),
                                                        None,
                                                        cx,
                                                    );
                                                });
                                            })
                                            .ok();
                                    }
                                })
                                .detach();

                                let mcp_registry = std::sync::Arc::new(
                                    crate::infrastructure::mcp::registry::McpToolRegistry::new(
                                        db.clone(),
                                    ),
                                );
                                let cancel_for_stream = cancel_flag_for_ai.clone();
                                let executor =
                                    crate::application::orchestration::executor::AgentExecutor::new(
                                        adapter,
                                        mcp_registry,
                                        db.clone(),
                                        team_bus_clone.clone(),
                                        instance_id_for_ai.clone(),
                                        agent.id.clone(),
                                        Some(session_id_for_ai.clone()),
                                        Some(run_id_for_ai.clone()),
                                        Some(cancel_flag_for_ai.clone()),
                                        Some(std::sync::Arc::new(move |partial| {
                                            if !cancel_for_stream.load(Ordering::SeqCst) {
                                                stream_tx.send_replace(partial);
                                            }
                                        })),
                                    );

                                let response_started_at = Instant::now();
                                match executor.execute_task(full_history).await {
                                    Ok(full_text) => {
                                        let thought_duration_secs =
                                            response_started_at.elapsed().as_secs_f64();
                                        let _ = db_clone
                                            .update_team_message_content(&office_msg_id, &full_text);
                                        if cancel_flag_for_ai.load(Ordering::SeqCst) {
                                            let _ = db_clone.update_team_message_delivery_status(
                                                &office_msg_id,
                                                "cancelled",
                                            );
                                        } else {
                                            let _ = db_clone.update_team_message_delivery_status(
                                                &office_msg_id,
                                                "delivered",
                                            );
                                        }

                                        let chat_service =
                                            crate::application::services::chat_service::ChatService::new(
                                                db_clone.clone(),
                                                team_bus_clone.clone(),
                                            );
                                        let (files_written, clean_text) = chat_service
                                            .parse_generated_response(
                                                &full_text,
                                                workspace_dir_for_ai.as_ref(),
                                            );
                                        if !files_written.is_empty() {
                                            let _ = db_clone.update_team_message_content(
                                                &office_msg_id,
                                                &clean_text,
                                            );
                                        }
                                        let chosen = if files_written.is_empty() {
                                            full_text
                                        } else {
                                            clean_text
                                        };
                                        round_result = Some(chosen.clone());
                                        let _ = db_clone.ensure_session(
                                            &session_id_for_ai,
                                            &agent.id,
                                            Some(&instance_id_for_ai),
                                        );
                                        let final_metadata = chat_message_metadata(
                                            &agent_name_str,
                                            Some(thought_duration_secs),
                                        );
                                        let _ = db_clone.append_conversation_turn(
                                            &session_id_for_ai,
                                            "assistant",
                                            round_result.as_ref().unwrap(),
                                            Some(&final_metadata),
                                        );
                                        let _ = db_clone.touch_session(&session_id_for_ai);

                                        let agent_name_for_office = agent.name.clone();
                                        let agent_id_for_office = agent.id.clone();
                                        let chosen_for_office = chosen;
                                        let _ = cx
                                            .update(|cx| {
                                                view.update(cx, |this: &mut Self, cx| {
                                                    this.update_chat_message_content(
                                                        &session_id_for_ai,
                                                        msg_idx,
                                                        chosen_for_office.clone(),
                                                        Some(thought_duration_secs),
                                                        cx,
                                                    );
                                                    this.push_office_chat_message(
                                                        &agent_id_for_office,
                                                        &chosen_for_office,
                                                        false,
                                                        &agent_name_for_office,
                                                        cx,
                                                    );
                                                })
                                            })
                                            .ok();
                                    }
                                    Err(e) => {
                                        let error_text = format!("Error: {}", e);
                                        let _ = db_clone
                                            .update_team_message_content(&office_msg_id, &error_text);
                                        let _ = db_clone.update_team_message_delivery_status(
                                            &office_msg_id,
                                            "failed",
                                        );
                                        let _ = cx
                                            .update(|cx| {
                                                view.update(cx, |this: &mut Self, cx| {
                                                    this.update_chat_message_content(
                                                        &session_id_for_ai,
                                                        msg_idx,
                                                        error_text.clone(),
                                                        None,
                                                        cx,
                                                    );
                                                })
                                            })
                                            .ok();
                                    }
                                }
                            }
                            if let Some(text) = round_result {
                                produced_response = true;
                                current_history.push(crate::providers::ChatMessage {
                                    role: gpui::SharedString::from("assistant"),
                                    content: gpui::SharedString::from(text.clone()),
                                    parts: vec![],
                                    agent_name: Some(gpui::SharedString::from(agent.name.clone())),
                                    thought_duration_secs: None,
                                });
                                if text.contains("[CONSENSUS_REACHED]") {
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            let has_pending_approval = db
                .list_pending_approval_requests(1000)
                .unwrap_or_default()
                .iter()
                .any(|request| request.run_id == run_id_for_ai);
            let run_status = if cancel_flag_for_ai.load(Ordering::SeqCst) {
                "cancelled"
            } else if has_pending_approval {
                "waiting_approval"
            } else if produced_response {
                "completed"
            } else {
                "failed"
            };
            let _ = db.update_orchestration_run_status(&run_id_for_ai, run_status, None);
            Self::finish_initial_iflow_for_run(&db, &run_id_for_ai, run_status);
            let _ = db.insert_run_event(&crate::core::models::RunEventRecord {
                id: uuid::Uuid::new_v4().to_string(),
                run_id: run_id_for_ai.clone(),
                event_type: format!("run_{}", run_status),
                actor_type: "system".to_string(),
                actor_id: None,
                task_id: None,
                payload: if use_mock {
                    Some("No usable provider completed the request".to_string())
                } else {
                    None
                },
                created_at: chrono::Utc::now().to_rfc3339(),
            });
            let _ = cx.update(|cx| {
                view.update(cx, |this: &mut Self, cx| {
                    this.is_generating = false;
                    this.generation_cancel_flag = None;
                    cx.notify();
                });
            });
            if use_mock {
                let _ = cx.update(|cx| {
                    view.update(cx, |this: &mut Self, cx| {
                        let error_text = "No valid provider found or configured for this team. Please check agent provider settings.";
                        {
                            let history = this
                                .chat_histories
                                .entry(session_id_for_ai.clone())
                                .or_default();
                            history.push(crate::providers::ChatMessage { role: "assistant".into(), content: error_text.into(), agent_name: None, thought_duration_secs: None , parts: vec![] });
                        }
                        this.rebuild_chat_display(&session_id_for_ai);
                        this.refresh_pending_chat_action(cx);
                        let assistant_msg = crate::teambus::routing::TeamMessage::new_broadcast(
                            instance_id_for_ai.clone(),
                            "assistant".to_string(),
                            error_text.to_string(),
                        );
                        let _ = crate::AppState::global(cx).db.insert_team_message(&assistant_msg);
                        let team_bus = this.team_bus.clone();
                        let assistant_msg_clone = assistant_msg.clone();
                        cx.spawn(async move |_, _| {
                            let _ = team_bus.route_message(assistant_msg_clone).await;
                        })
                        .detach();
                        let display_len = this
                            .chat_display_rows
                            .get(&session_id_for_ai)
                            .map(|v| v.len())
                            .unwrap_or(0);
                        this.chat_list_state = gpui::ListState::new(display_len, gpui::ListAlignment::Bottom, px(200.));
                        cx.notify();
                    });
                });
            }
        }).detach();
        }
    }
    pub(crate) fn refresh_pending_chat_action(&mut self, cx: &mut Context<Self>) {
        let Some(instance_id) = self.selected_instance_id.clone() else {
            self.pending_chat_action = None;
            return;
        };
        let db = crate::AppState::global(cx).db.clone();

        for request in db.list_pending_approval_requests(100).unwrap_or_default() {
            let Some(run) = db.get_orchestration_run(&request.run_id).ok().flatten() else {
                continue;
            };
            if run.instance_id != instance_id {
                continue;
            }

            let details = approval_operation_details(&request.operation);
            let tool_name = if details.tool_name.is_empty() {
                "tool".to_string()
            } else {
                details.tool_name
            };
            let risk_label = approval_risk_label(&tool_name, details.path.as_deref());
            self.pending_chat_action = Some(PendingChatAction::Approval {
                request_id: request.id,
                run_id: request.run_id,
                tool_name,
                command: details.command,
                path: details.path,
                risk_label,
                mode_label: details.mode.or(Some(run.mode)),
                requested_at: request.created_at,
            });
            return;
        }

        let pending_readback = db
            .list_recent_collaboration_cases(25)
            .unwrap_or_default()
            .into_iter()
            .filter(|case_record| {
                case_record.owner_instance_id == instance_id
                    || case_record.target_instance_id == instance_id
            })
            .find_map(|case_record| {
                db.list_case_readbacks(&case_record.id)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|readback| readback.status == "submitted")
                    .last()
                    .map(|readback| (case_record, readback))
            });

        self.pending_chat_action =
            pending_readback.map(|(case_record, readback)| PendingChatAction::Readback {
                case_id: case_record.id,
                readback_id: readback.id,
                origin_run_id: case_record.origin_run_id,
                objective: case_record.objective,
            });
    }

    fn render_cached_pending_action_summary(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(action) = self.pending_chat_action.clone() else {
            return div().into_any_element();
        };
        let theme = cx.theme().clone();

        match action {
            PendingChatAction::Approval {
                request_id,
                run_id,
                tool_name,
                command,
                path,
                risk_label,
                mode_label,
                requested_at,
            } => {
                let approve_id = request_id.clone();
                let approve_run_id = run_id.clone();
                let reject_id = request_id.clone();
                let reject_run_id = run_id.clone();

                div()
                    .w_full()
                    .px(px(14.))
                    .pb(px(8.))
                    .child(
                        div()
                            .w_full()
                            .rounded_md()
                            .border_1()
                            .border_color(gpui::yellow().opacity(0.45))
                            .bg(gpui::yellow().opacity(0.08))
                            .p_3()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                Icon::new(IconName::SquareTerminal)
                                                    .size(px(14.))
                                                    .text_color(gpui::yellow()),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .child("Tool approval required"),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .px(px(8.))
                                            .py(px(3.))
                                            .rounded_full()
                                            .bg(gpui::yellow().opacity(0.16))
                                            .text_color(gpui::yellow())
                                            .text_size(px(11.))
                                            .child(risk_label),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(format!("tool: {}", tool_name))
                                    .when_some(mode_label, |this, mode| {
                                        this.child(format!("mode: {}", mode))
                                    })
                                    .child(format!("requested: {}", requested_at))
                                    .when_some(path, |this, path| {
                                        this.child(format!(
                                            "path: {}",
                                            compact_for_card(&path, 180)
                                        ))
                                    })
                                    .when_some(command, |this, command| {
                                        this.child(format!(
                                            "command: {}",
                                            compact_for_card(&command, 180)
                                        ))
                                    }),
                            )
                            .child(
                                h_flex()
                                    .justify_end()
                                    .gap_2()
                                    .child(
                                        Button::new(gpui::SharedString::from(format!(
                                            "chat-cached-approve-{}",
                                            approve_id
                                        )))
                                        .small()
                                        .primary()
                                        .label("Approve")
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            let state = crate::AppState::global(cx);
                                            let db = state.db.clone();
                                            let actor_id = state.current_actor_id.clone();
                                            let _ = db.resolve_approval_request(
                                                &approve_id,
                                                "approved",
                                                Some(&actor_id),
                                                Some("Approved from chat action card"),
                                            );
                                            let _ = db.resolve_waiting_tasks_for_run(
                                                &approve_run_id,
                                                "pending",
                                            );
                                            let _ = db.update_orchestration_run_status(
                                                &approve_run_id,
                                                "running",
                                                None,
                                            );
                                            let _ = db.insert_run_event(
                                                &crate::core::models::RunEventRecord {
                                                    id: uuid::Uuid::new_v4().to_string(),
                                                    run_id: approve_run_id.clone(),
                                                    event_type: "tool_approval_approved"
                                                        .to_string(),
                                                    actor_type: "user".to_string(),
                                                    actor_id: Some(actor_id),
                                                    task_id: None,
                                                    payload: None,
                                                    created_at: chrono::Utc::now().to_rfc3339(),
                                                },
                                            );
                                            let _ = crate::application::iflow_engine::automation::IFlowAutomation::resume_approved_run(
                                                db,
                                                state.team_bus.clone(),
                                                state.tokio_runtime.clone(),
                                                &approve_run_id,
                                            );
                                            this.refresh_pending_chat_action(cx);
                                            cx.notify();
                                        })),
                                    )
                                    .child(
                                        Button::new(gpui::SharedString::from(format!(
                                            "chat-cached-reject-{}",
                                            reject_id
                                        )))
                                        .small()
                                        .label("Reject")
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            let state = crate::AppState::global(cx);
                                            let db = state.db.clone();
                                            let actor_id = state.current_actor_id.clone();
                                            let _ = db.resolve_approval_request(
                                                &reject_id,
                                                "rejected",
                                                Some(&actor_id),
                                                Some("Rejected from chat action card"),
                                            );
                                            let _ = db.resolve_waiting_tasks_for_run(
                                                &reject_run_id,
                                                "failed",
                                            );
                                            let _ = db.update_orchestration_run_status(
                                                &reject_run_id,
                                                "failed",
                                                None,
                                            );
                                            let _ = db.insert_run_event(
                                                &crate::core::models::RunEventRecord {
                                                    id: uuid::Uuid::new_v4().to_string(),
                                                    run_id: reject_run_id.clone(),
                                                    event_type: "tool_approval_rejected"
                                                        .to_string(),
                                                    actor_type: "user".to_string(),
                                                    actor_id: Some(actor_id),
                                                    task_id: None,
                                                    payload: None,
                                                    created_at: chrono::Utc::now().to_rfc3339(),
                                                },
                                            );
                                            let _ = crate::application::iflow_engine::automation::IFlowAutomation::reject_waiting_run(
                                                db,
                                                state.team_bus.clone(),
                                                &reject_run_id,
                                            );
                                            this.refresh_pending_chat_action(cx);
                                            cx.notify();
                                        })),
                                    ),
                            ),
                    )
                    .into_any_element()
            }
            PendingChatAction::Readback {
                case_id,
                readback_id,
                origin_run_id,
                objective,
            } => {
                let accept_case_id = case_id.clone();
                let accept_readback_id = readback_id.clone();
                let answer_prompt = "Tra loi case:\n- Cau tra loi: ".to_string();
                let label = compact_for_card(&objective, 140);
                div()
                    .w_full()
                    .px(px(14.))
                    .pb(px(8.))
                    .child(
                        h_flex()
                            .w_full()
                            .rounded_md()
                            .border_1()
                            .border_color(gpui::blue().opacity(0.35))
                            .bg(gpui::blue().opacity(0.06))
                            .p_3()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .min_w_0()
                                    .child(Icon::new(IconName::Building2).size(px(14.)))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .min_w_0()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .child("Readback awaiting acceptance"),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(label),
                                            ),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .flex_none()
                                    .child(
                                        Button::new(gpui::SharedString::from(format!(
                                            "chat-cached-accept-readback-{}",
                                            accept_readback_id
                                        )))
                                        .small()
                                        .primary()
                                        .label("Accept")
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            let state = crate::AppState::global(cx);
                                            let service =
                                                crate::application::orchestration::collaboration::CollaborationService::new(
                                                    state.db.clone(),
                                                );
                                            let _ = service.accept_readback(
                                                &accept_case_id,
                                                &accept_readback_id,
                                                &state.current_actor_id,
                                            );
                                            if let Some(run_id) = origin_run_id.as_deref() {
                                                let _ = state.db.update_orchestration_run_status(
                                                    run_id,
                                                    "running",
                                                    None,
                                                );
                                                let _ = state
                                                    .db
                                                    .resolve_waiting_tasks_for_run(run_id, "pending");
                                                let _ = crate::application::iflow_engine::automation::IFlowAutomation::resume_approved_run(
                                                    state.db.clone(),
                                                    state.team_bus.clone(),
                                                    state.tokio_runtime.clone(),
                                                    run_id,
                                                );
                                            }
                                            this.refresh_pending_chat_action(cx);
                                            cx.notify();
                                        })),
                                    )
                                    .child(
                                        Button::new(gpui::SharedString::from(format!(
                                            "chat-cached-answer-readback-{}",
                                            readback_id
                                        )))
                                        .small()
                                        .label("Answer")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.chat_input_state.update(cx, |state, cx| {
                                                state.set_value(&answer_prompt, window, cx);
                                            });
                                            cx.notify();
                                        })),
                                    ),
                            ),
                    )
                    .into_any_element()
            }
        }
    }

    #[allow(dead_code)]
    fn render_pending_action_summary_legacy(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(instance_id) = self.selected_instance_id.clone() else {
            return div().into_any_element();
        };
        let theme = cx.theme().clone();
        let db = crate::AppState::global(cx).db.clone();

        let pending_approval = db
            .list_pending_approval_requests(50)
            .unwrap_or_default()
            .into_iter()
            .find(|request| {
                db.get_orchestration_run(&request.run_id)
                    .ok()
                    .flatten()
                    .is_some_and(|run| run.instance_id == instance_id)
            });

        if let Some(request) = pending_approval {
            let approve_id = request.id.clone();
            let approve_run_id = request.run_id.clone();
            let reject_id = request.id.clone();
            let reject_run_id = request.run_id.clone();
            let label = compact_for_card(&request.operation, 120);
            return div()
                .w_full()
                .px(px(14.))
                .pb(px(8.))
                .child(
                    h_flex()
                        .w_full()
                        .rounded_md()
                        .border_1()
                        .border_color(gpui::yellow().opacity(0.45))
                        .bg(gpui::yellow().opacity(0.08))
                        .p_3()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .min_w_0()
                                .child(Icon::new(IconName::SquareTerminal).size(px(14.)))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .min_w_0()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .child("Tool approval required"),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(theme.muted_foreground)
                                                .child(label),
                                        ),
                                ),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .flex_none()
                                .child(
                                    Button::new(gpui::SharedString::from(format!(
                                        "chat-summary-approve-{}",
                                        approve_id
                                    )))
                                    .small()
                                    .primary()
                                    .label("Approve")
                                    .on_click(cx.listener(move |_this, _, _, cx| {
                                        let state = crate::AppState::global(cx);
                                        let db = state.db.clone();
                                        let _ = db.resolve_approval_request(
                                            &approve_id,
                                            "approved",
                                            Some(&state.current_actor_id),
                                            Some("Approved from chat action summary"),
                                        );
                                        let _ = db.resolve_waiting_tasks_for_run(
                                            &approve_run_id,
                                            "pending",
                                        );
                                        let _ = db.update_orchestration_run_status(
                                            &approve_run_id,
                                            "running",
                                            None,
                                        );
                                        let _ = crate::application::iflow_engine::automation::IFlowAutomation::resume_approved_run(
                                            db,
                                            state.team_bus.clone(),
                                            state.tokio_runtime.clone(),
                                            &approve_run_id,
                                        );
                                        cx.notify();
                                    })),
                                )
                                .child(
                                    Button::new(gpui::SharedString::from(format!(
                                        "chat-summary-reject-{}",
                                        reject_id
                                    )))
                                    .small()
                                    .label("Reject")
                                    .on_click(cx.listener(move |_this, _, _, cx| {
                                        let state = crate::AppState::global(cx);
                                        let db = state.db.clone();
                                        let _ = db.resolve_approval_request(
                                            &reject_id,
                                            "rejected",
                                            Some(&state.current_actor_id),
                                            Some("Rejected from chat action summary"),
                                        );
                                        let _ =
                                            db.resolve_waiting_tasks_for_run(&reject_run_id, "failed");
                                        let _ = db.update_orchestration_run_status(
                                            &reject_run_id,
                                            "failed",
                                            None,
                                        );
                                        let _ = crate::application::iflow_engine::automation::IFlowAutomation::reject_waiting_run(
                                            db,
                                            state.team_bus.clone(),
                                            &reject_run_id,
                                        );
                                        cx.notify();
                                    })),
                                ),
                        ),
                )
                .into_any_element();
        }

        let pending_readback = db
            .list_recent_collaboration_cases(25)
            .unwrap_or_default()
            .into_iter()
            .filter(|case_record| {
                case_record.owner_instance_id == instance_id
                    || case_record.target_instance_id == instance_id
            })
            .find_map(|case_record| {
                db.list_case_readbacks(&case_record.id)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|readback| readback.status == "submitted")
                    .last()
                    .map(|readback| (case_record, readback))
            });

        if let Some((case_record, readback)) = pending_readback {
            let accept_case_id = case_record.id.clone();
            let accept_readback_id = readback.id.clone();
            let origin_run_id = case_record.origin_run_id.clone();
            let answer_prompt = "Tra loi case:\n- Cau tra loi: ".to_string();
            let label = compact_for_card(&case_record.objective, 140);
            return div()
                .w_full()
                .px(px(14.))
                .pb(px(8.))
                .child(
                    h_flex()
                        .w_full()
                        .rounded_md()
                        .border_1()
                        .border_color(gpui::blue().opacity(0.35))
                        .bg(gpui::blue().opacity(0.06))
                        .p_3()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .min_w_0()
                                .child(Icon::new(IconName::Building2).size(px(14.)))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .min_w_0()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .child("Readback awaiting acceptance"),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(theme.muted_foreground)
                                                .child(label),
                                        ),
                                ),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .flex_none()
                                .child(
                                    Button::new(gpui::SharedString::from(format!(
                                        "chat-summary-accept-readback-{}",
                                        accept_readback_id
                                    )))
                                    .small()
                                    .primary()
                                    .label("Accept")
                                    .on_click(cx.listener(move |_this, _, _, cx| {
                                        let state = crate::AppState::global(cx);
                                        let service =
                                            crate::application::orchestration::collaboration::CollaborationService::new(
                                                state.db.clone(),
                                            );
                                        let _ = service.accept_readback(
                                            &accept_case_id,
                                            &accept_readback_id,
                                            &state.current_actor_id,
                                        );
                                        if let Some(run_id) = origin_run_id.as_deref() {
                                            let _ = state.db.update_orchestration_run_status(
                                                run_id,
                                                "running",
                                                None,
                                            );
                                            let _ = state
                                                .db
                                                .resolve_waiting_tasks_for_run(run_id, "pending");
                                            let _ = crate::application::iflow_engine::automation::IFlowAutomation::resume_approved_run(
                                                state.db.clone(),
                                                state.team_bus.clone(),
                                                state.tokio_runtime.clone(),
                                                run_id,
                                            );
                                        }
                                        cx.notify();
                                    })),
                                )
                                .child(
                                    Button::new(gpui::SharedString::from(format!(
                                        "chat-summary-answer-readback-{}",
                                        readback.id
                                    )))
                                    .small()
                                    .label("Answer")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.chat_input_state.update(cx, |state, cx| {
                                            state.set_value(&answer_prompt, window, cx);
                                        });
                                        cx.notify();
                                    })),
                                ),
                        ),
                )
                .into_any_element();
        }

        div().into_any_element()
    }

    #[allow(dead_code)]
    fn render_pending_action_cards_legacy(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(instance_id) = self.selected_instance_id.clone() else {
            return div().into_any_element();
        };
        let theme = cx.theme().clone();
        let db = crate::AppState::global(cx).db.clone();
        let mut cards = div()
            .w_full()
            .px(px(14.))
            .pb(px(8.))
            .flex()
            .flex_col()
            .gap_2();
        let mut count = 0usize;

        for request in db
            .list_pending_approval_requests(1000)
            .unwrap_or_default()
            .into_iter()
            .filter(|request| {
                db.get_orchestration_run(&request.run_id)
                    .ok()
                    .flatten()
                    .is_some_and(|run| run.instance_id == instance_id)
            })
            .take(2)
        {
            count += 1;
            let approve_id = request.id.clone();
            let approve_run_id = request.run_id.clone();
            let reject_id = request.id.clone();
            let reject_run_id = request.run_id.clone();
            let operation_label = compact_for_card(&request.operation, 140);

            cards = cards.child(
                div()
                    .w_full()
                    .rounded_md()
                    .border_1()
                    .border_color(gpui::yellow().opacity(0.45))
                    .bg(gpui::yellow().opacity(0.08))
                    .p_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .min_w_0()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        Icon::new(IconName::SquareTerminal)
                                            .size(px(14.))
                                            .text_color(gpui::yellow()),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child("Tool approval required"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(operation_label),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .flex_none()
                            .child(
                                Button::new(gpui::SharedString::from(format!(
                                    "chat-approve-{}",
                                    approve_id
                                )))
                                .small()
                                .primary()
                                .label("Approve")
                                .on_click(cx.listener(move |_this, _, _, cx| {
                                    let state = crate::AppState::global(cx);
                                    let db = state.db.clone();
                                    let team_bus = state.team_bus.clone();
                                    let runtime = state.tokio_runtime.clone();
                                    let actor_id = state.current_actor_id.clone();
                                    let _ = db.resolve_approval_request(
                                        &approve_id,
                                        "approved",
                                        Some(&actor_id),
                                        Some("Approved from chat action card"),
                                    );
                                    let _ = db
                                        .resolve_waiting_tasks_for_run(&approve_run_id, "pending");
                                    let _ = db.update_orchestration_run_status(
                                        &approve_run_id,
                                        "running",
                                        None,
                                    );
                                    let _ = crate::application::iflow_engine::automation::IFlowAutomation::resume_approved_run(
                                        db,
                                        team_bus,
                                        runtime,
                                        &approve_run_id,
                                    );
                                    cx.notify();
                                })),
                            )
                            .child(
                                Button::new(gpui::SharedString::from(format!(
                                    "chat-reject-{}",
                                    reject_id
                                )))
                                .small()
                                .label("Reject")
                                .on_click(cx.listener(move |_this, _, _, cx| {
                                    let state = crate::AppState::global(cx);
                                    let db = state.db.clone();
                                    let team_bus = state.team_bus.clone();
                                    let actor_id = state.current_actor_id.clone();
                                    let _ = db.resolve_approval_request(
                                        &reject_id,
                                        "rejected",
                                        Some(&actor_id),
                                        Some("Rejected from chat action card"),
                                    );
                                    let _ =
                                        db.resolve_waiting_tasks_for_run(&reject_run_id, "failed");
                                    let _ = db.update_orchestration_run_status(
                                        &reject_run_id,
                                        "failed",
                                        None,
                                    );
                                    let _ = crate::application::iflow_engine::automation::IFlowAutomation::reject_waiting_run(
                                        db,
                                        team_bus,
                                        &reject_run_id,
                                    );
                                    cx.notify();
                                })),
                            ),
                    ),
            );
        }

        for case_record in db
            .list_recent_collaboration_cases(100)
            .unwrap_or_default()
            .into_iter()
            .filter(|case_record| {
                case_record.owner_instance_id == instance_id
                    || case_record.target_instance_id == instance_id
            })
            .take(50)
        {
            let Some(readback) = db
                .list_case_readbacks(&case_record.id)
                .unwrap_or_default()
                .into_iter()
                .filter(|readback| readback.status == "submitted")
                .last()
            else {
                continue;
            };
            count += 1;
            if count > 4 {
                break;
            }

            let accept_case_id = case_record.id.clone();
            let accept_readback_id = readback.id.clone();
            let origin_run_id = case_record.origin_run_id.clone();
            let objective = compact_for_card(&case_record.objective, 150);
            let understanding = compact_for_card(&readback.understanding, 150);
            let questions = readback_questions_label(&readback.questions_json);
            let answer_prompt = "Trả lời case:\n- Câu trả lời: ".to_string();

            cards = cards.child(
                div()
                    .w_full()
                    .rounded_md()
                    .border_1()
                    .border_color(gpui::blue().opacity(0.35))
                    .bg(gpui::blue().opacity(0.06))
                    .p_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .min_w_0()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        Icon::new(IconName::Building2)
                                            .size(px(14.))
                                            .text_color(gpui::blue()),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child("Readback awaiting acceptance"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(objective),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.foreground)
                                    .child(format!("Understanding: {}", understanding)),
                            )
                            .when_some(questions, |this, questions| {
                                this.child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(theme.muted_foreground)
                                        .child(format!("Questions: {}", questions)),
                                )
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .flex_none()
                            .child(
                                Button::new(gpui::SharedString::from(format!(
                                    "chat-accept-readback-{}",
                                    accept_readback_id
                                )))
                                .small()
                                .primary()
                                .label("Accept")
                                .on_click(cx.listener(move |_this, _, _, cx| {
                                    let state = crate::AppState::global(cx);
                                    let service =
                                        crate::application::orchestration::collaboration::CollaborationService::new(
                                            state.db.clone(),
                                        );
                                    let _ = service.accept_readback(
                                        &accept_case_id,
                                        &accept_readback_id,
                                        &state.current_actor_id,
                                    );
                                    if let Some(run_id) = origin_run_id.as_deref() {
                                        let _ = state.db.update_orchestration_run_status(
                                            run_id,
                                            "running",
                                            None,
                                        );
                                        let _ =
                                            state.db.resolve_waiting_tasks_for_run(run_id, "pending");
                                        let _ = crate::application::iflow_engine::automation::IFlowAutomation::resume_approved_run(
                                            state.db.clone(),
                                            state.team_bus.clone(),
                                            state.tokio_runtime.clone(),
                                            run_id,
                                        );
                                    }
                                    cx.notify();
                                })),
                            )
                            .child(
                                Button::new(gpui::SharedString::from(format!(
                                    "chat-answer-readback-{}",
                                    readback.id
                                )))
                                .small()
                                .label("Answer")
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.chat_input_state.update(cx, |state, cx| {
                                        state.set_value(&answer_prompt, window, cx);
                                    });
                                    cx.notify();
                                })),
                            ),
                    ),
            );
        }

        if count == 0 {
            return div().into_any_element();
        }
        cards.into_any_element()
    }

    pub(crate) fn render_entry(
        &mut self,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let session_id = if let Some(id) = &self.selected_session_id {
            id.clone()
        } else {
            return div().into_any_element();
        };

        if !self.chat_histories.contains_key(&session_id) {
            return div().into_any_element();
        }
        let Some(rows) = self.chat_display_rows.get(&session_id) else {
            return div().into_any_element();
        };
        if rows.is_empty() || ix >= rows.len() {
            return div().into_any_element();
        }
        let row = rows[ix].clone();
        let theme = cx.theme().clone();

        let (source_index, msg) = match row {
            super::ChatDisplayRow::CrossTeamThreadHeader {
                correlation_id,
                handoff_type,
                from_team,
                count,
                preview,
                has_request,
                has_response,
            } => {
                let is_expanded = self.expanded_threads.contains(&correlation_id);
                let icon = if is_expanded {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                };
                let session_id_clone = session_id.clone();
                let correlation_id_clone = correlation_id.clone();
                let from_team_label = self
                    .instances
                    .iter()
                    .find(|i| i.id == from_team)
                    .map(|i| i.name.clone())
                    .unwrap_or(from_team);
                let from_team_short = {
                    let mut end = from_team_label.len();
                    let mut chars = 0usize;
                    for (i, _) in from_team_label.char_indices() {
                        if chars == 12 {
                            end = i;
                            break;
                        }
                        chars += 1;
                    }
                    if chars >= 12 && end < from_team_label.len() {
                        format!("{}...", &from_team_label[..end])
                    } else {
                        from_team_label
                    }
                };
                // Smart badge: 3 states based on has_request + has_response
                let (status_color, status_label) = if has_response {
                    (gpui::green(), "✓ Responded")
                } else if has_request {
                    (gpui::yellow(), "⏳ Pending Review")
                } else {
                    (gpui::blue(), "Handoff")
                };
                let thread_card = div()
                    .id(("cross-team-thread", ix))
                    .w_full()
                    .px(px(12.))
                    .py(px(8.))
                    .rounded_md()
                    .bg(status_color.opacity(0.06))
                    .border(px(1.))
                    .border_color(status_color.opacity(0.25))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.expanded_threads.contains(&correlation_id_clone) {
                            this.expanded_threads.remove(&correlation_id_clone);
                        } else {
                            this.expanded_threads.insert(correlation_id_clone.clone());
                        }
                        this.rebuild_chat_display(&session_id_clone);
                        let display_len = this
                            .chat_display_rows
                            .get(&session_id_clone)
                            .map(|v| v.len())
                            .unwrap_or(0);
                        this.chat_list_state = gpui::ListState::new(
                            display_len,
                            gpui::ListAlignment::Bottom,
                            px(200.),
                        );
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .child(
                                                Icon::new(icon)
                                                    .size(px(14.))
                                                    .text_color(theme.muted_foreground),
                                            )
                                            .child(
                                                div()
                                                    .font_weight(gpui::FontWeight::BOLD)
                                                    .text_size(px(13.))
                                                    .child(format!(
                                                        "Cross-team {} ({})",
                                                        handoff_type, count
                                                    )),
                                            ),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .child(
                                                div()
                                                    .px(px(8.))
                                                    .py(px(3.))
                                                    .rounded_full()
                                                    .bg(status_color.opacity(0.14))
                                                    .text_color(status_color)
                                                    .text_size(px(11.))
                                                    .child(status_label),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(from_team_short),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(preview),
                            ),
                    );
                return div()
                    .w_full()
                    .px(px(4.))
                    .py(px(5.))
                    .child(thread_card)
                    .into_any_element();
            }
            super::ChatDisplayRow::Message { source_index, msg } => (source_index, msg),
        };

        let is_user = msg.role == "user";

        let msg_key = format!("{}_{}", session_id, source_index);
        let is_expanded = self.expanded_messages.contains(&msg_key);

        let line_count = msg.content.lines().count();
        let is_large_message = line_count > CHAT_COLLAPSE_LINE_LIMIT
            || exceeds_char_limit(msg.content.as_ref(), CHAT_COLLAPSE_CHAR_LIMIT);
        let needs_collapse = (is_user && line_count > 5) || (!is_user && is_large_message);

        let content_to_render = if needs_collapse && !is_expanded {
            if is_user {
                let first_line = msg.content.lines().next().unwrap_or("");
                format!("{} ...", first_line)
            } else {
                truncate_chars_with_marker(
                    msg.content.as_ref(),
                    CHAT_COLLAPSE_CHAR_LIMIT,
                    "\n\n[content collapsed for render safety; expand to view more]",
                )
                .unwrap_or_else(|| msg.content.to_string())
            }
        } else {
            msg.content.to_string()
        };

        let mut display_name = if let Some(name) = &msg.agent_name {
            name.to_string()
        } else {
            "Agent".to_string()
        };
        let mut display_content = content_to_render.clone();
        let is_thinking =
            !is_user && msg.thought_duration_secs.is_none() && display_content.trim().is_empty();
        if is_thinking {
            display_content = AI_THINKING_LABEL.to_string();
        }
        let thought_duration_secs = if !is_user && !is_thinking {
            msg.thought_duration_secs
        } else {
            None
        };

        if !is_user {
            // Strip legacy prefixes like [Task Completed] {task_id}:

            if display_content.starts_with("[Task Completed]") {
                if let Some(idx) = display_content.find(":\n") {
                    display_content = display_content[idx + 2..].to_string();
                }
            }
            // Strip legacy interactive [AgentName]:
            else if let Some(end_bracket) = display_content.find("]: ") {
                if display_content.starts_with('[') {
                    if msg.agent_name.is_none() {
                        display_name = display_content[1..end_bracket].to_string();
                    }
                    display_content = display_content[end_bracket + 3..].to_string();
                }
            }

            if display_content.starts_with("[CROSS_TEAM_HANDOFF]") {
                let payload_str = display_content
                    .trim_start_matches("[CROSS_TEAM_HANDOFF]")
                    .trim();
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload_str) {
                    let handoff_type = v
                        .get("handoff_type")
                        .and_then(|x| x.as_str())
                        .unwrap_or("handoff");
                    let correlation_id = v
                        .get("correlation_id")
                        .and_then(|x| x.as_str())
                        .unwrap_or("");
                    let from_team = v.get("from_team").and_then(|x| x.as_str()).unwrap_or("");
                    let package = v
                        .get("briefing_package")
                        .and_then(|x| x.as_str())
                        .unwrap_or("");
                    display_name = format!("Cross-team {}", handoff_type);
                    display_content = format!(
                        "correlation_id: {}\nfrom_team: {}\n\n{}",
                        correlation_id, from_team, package
                    );
                }
            }
        }

        if let Some(truncated) = truncate_chars_with_marker(
            &display_content,
            CHAT_RENDER_CHAR_LIMIT,
            "\n\n[truncated for render safety]",
        ) {
            display_content = truncated;
        }

        let avatar_bg = agent_avatar_color(&display_name);
        let agent_avatar = div()
            .w(px(28.))
            .h(px(28.))
            .rounded_full()
            .bg(avatar_bg)
            .flex()
            .items_center()
            .justify_center()
            .child(Icon::empty().path("icons/bot.svg").size(px(16.)));

        let text_element = if is_user {
            let mut user_theme = theme.clone();
            user_theme.foreground = gpui::Hsla::from(gpui::rgb(0x111827));
            user_theme.muted_foreground = gpui::Hsla::from(gpui::rgb(0x374151));
            user_theme.border = gpui::Hsla::from(gpui::rgba(0x11182733));
            Self::render_message_text(&display_content, &user_theme, window)
        } else {
            Self::render_message_text(&display_content, &theme, window)
        };

        let elem = if is_user {
            let collapse_key = msg_key.clone();
            h_flex()
                .id(("msg-row", ix))
                .group("msg_row")
                .w_full()
                .justify_end()
                .items_start()
                .gap_2()
                .child(
                    h_flex()
                        .invisible()
                        .group_hover("msg_row", |s| s.visible())
                        .gap_1()
                        .child(
                            div()
                                .id(("delete-msg", ix))
                                .cursor_pointer()
                                .text_color(theme.muted_foreground)
                                .hover(|s| s.text_color(gpui::red()))
                                .child(Icon::empty().path("icons/trash.svg").size_8())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if let Some(session_id) = this.selected_session_id.clone() {
                                        if let Some(history) =
                                            this.chat_histories.get_mut(&session_id)
                                        {
                                            if source_index < history.len() {
                                                history.remove(source_index);
                                                this.rebuild_chat_display(&session_id);
                                                let display_len = this
                                                    .chat_display_rows
                                                    .get(&session_id)
                                                    .map(|v| v.len())
                                                    .unwrap_or(0);
                                                this.chat_list_state = gpui::ListState::new(
                                                    display_len,
                                                    gpui::ListAlignment::Bottom,
                                                    px(200.),
                                                );
                                                cx.notify();
                                            }
                                        }
                                    }
                                })),
                        )
                        // Clipboard
                        .child(
                            gpui_component::clipboard::Clipboard::new(("clipboard", ix))
                                .value(msg.content.clone().to_string()),
                        ),
                )
                .child(
                    div()
                        .max_w(px(640.0))
                        // .ml_auto()
                        .p_2()
                        .bg(gpui::Hsla::from(gpui::rgb(0xadecf9)))
                        .border_1()
                        .border_color(gpui::Hsla::from(gpui::rgba(0x11182722)))
                        .rounded_lg()
                        .overflow_hidden()
                        .flex()
                        .child(text_element)
                        .when(needs_collapse, |d: gpui::Div| {
                            let collapse_key = collapse_key.clone();
                            d.child(
                                div()
                                    .id(("collapse-btn", ix))
                                    .cursor_pointer()
                                    .flex_none()
                                    .text_color(theme.muted_foreground)
                                    .child(if is_expanded {
                                        IconName::ChevronUp
                                    } else {
                                        IconName::ChevronDown
                                    })
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if this.expanded_messages.contains(&collapse_key) {
                                            this.expanded_messages.remove(&collapse_key);
                                        } else {
                                            this.expanded_messages.insert(collapse_key.clone());
                                        }
                                        cx.notify();
                                    })),
                            )
                        }),
                )
                .into_any_element()
        } else {
            let collapse_key = msg_key.clone();
            div()
                .w_full()
                .flex()
                .justify_start()
                .child(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .items_start()
                        .child(agent_avatar)
                        .child(
                            div()
                                .max_w(gpui::relative(0.85))
                                .flex()
                                .flex_col()
                                .overflow_hidden()
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(theme.muted_foreground)
                                        .mb(px(2.))
                                        .child(display_name),
                                )
                                .child(
                                    div()
                                        .w_full()
                                        .rounded_lg()
                                        .overflow_hidden()
                                        .child(text_element),
                                )
                                .when(needs_collapse, |this| {
                                    let collapse_key = collapse_key.clone();
                                    this.child(
                                        div()
                                            .id(("assistant-collapse-btn", ix))
                                            .mt(px(6.))
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .cursor_pointer()
                                            .text_size(px(12.))
                                            .text_color(theme.muted_foreground)
                                            .child(if is_expanded {
                                                IconName::ChevronUp
                                            } else {
                                                IconName::ChevronDown
                                            })
                                            .child(if is_expanded {
                                                "Show less"
                                            } else {
                                                "Show more"
                                            })
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                if this.expanded_messages.contains(&collapse_key) {
                                                    this.expanded_messages.remove(&collapse_key);
                                                } else {
                                                    this.expanded_messages
                                                        .insert(collapse_key.clone());
                                                }
                                                cx.notify();
                                            })),
                                    )
                                })
                                .when_some(thought_duration_secs, |this, seconds| {
                                    this.child(
                                        div()
                                            .mt(px(4.))
                                            .text_size(px(11.))
                                            .text_color(theme.muted_foreground)
                                            .child(format!(
                                                "Thought for {}",
                                                format_thought_duration(seconds)
                                            )),
                                    )
                                }),
                        ),
                )
                .into_any_element()
        };

        div().p_2().child(elem).into_any_element()
    }

    fn render_message_text(
        content: &str,
        theme: &gpui_component::Theme,
        cx: &mut Window,
    ) -> gpui::AnyElement {
        let safe_content = truncate_chars_with_marker(
            content,
            CHAT_RENDER_CHAR_LIMIT,
            "\n\n[truncated for render safety]",
        );
        let content = safe_content.as_deref().unwrap_or(content);

        if let Some(html) = extract_renderable_html(content) {
            return crate::ui::text::html(html)
                .selectable(true)
                .into_any_element();
        }
        render_markdown_message(content, theme, cx).into_any_element()
    }
}

fn exceeds_char_limit(value: &str, limit: usize) -> bool {
    value.chars().nth(limit).is_some()
}

fn truncate_chars_with_marker(value: &str, max_chars: usize, marker: &str) -> Option<String> {
    let mut chars = value.chars();
    let mut out = String::new();
    for _ in 0..max_chars {
        let Some(ch) = chars.next() else {
            return None;
        };
        out.push(ch);
    }
    if chars.next().is_none() {
        return None;
    }
    out.push_str(marker);
    Some(out)
}

#[derive(Default)]
struct ApprovalOperationDetails {
    tool_name: String,
    command: Option<String>,
    path: Option<String>,
    mode: Option<String>,
}

fn approval_operation_details(operation: &str) -> ApprovalOperationDetails {
    let mut details = ApprovalOperationDetails::default();
    details.mode = operation
        .rsplit_once(":mode=")
        .map(|(_, mode)| mode.trim().to_string())
        .filter(|mode| !mode.is_empty());

    let Some(rest) = operation.strip_prefix("tool:") else {
        return details;
    };
    let Some((tool_name, tail)) = rest.split_once(':') else {
        details.tool_name = rest.trim().to_string();
        return details;
    };
    details.tool_name = tool_name.trim().to_string();

    let summary = if let Some((summary, _)) = tail.split_once(":payload=") {
        summary
    } else if tail.starts_with("payload=") {
        ""
    } else {
        tail.split(":mode=").next().unwrap_or(tail)
    };

    for part in summary
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if let Some(path) = part.strip_prefix("path=") {
            details.path = Some(path.trim().to_string()).filter(|path| !path.is_empty());
        } else if let Some(command) = part.strip_prefix("command=") {
            details.command =
                Some(command.trim().to_string()).filter(|command| !command.is_empty());
        }
    }

    details
}

fn approval_risk_label(tool_name: &str, path: Option<&str>) -> String {
    match crate::application::orchestration::tool_gateway::ToolExecutionGateway::risk_for(
        tool_name, false,
    ) {
        crate::application::orchestration::tool_gateway::ToolRisk::ReadOnly if path.is_some() => {
            "external path".to_string()
        }
        crate::application::orchestration::tool_gateway::ToolRisk::ReadOnly => {
            "read-only".to_string()
        }
        crate::application::orchestration::tool_gateway::ToolRisk::ControlledMutation => {
            "controlled mutation".to_string()
        }
        crate::application::orchestration::tool_gateway::ToolRisk::Sensitive => {
            "sensitive".to_string()
        }
    }
}

fn extract_renderable_html(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("```html") && trimmed.ends_with("```") {
        let first_newline = trimmed.find('\n')?;
        let body = &trimmed[first_newline + 1..trimmed.len().saturating_sub(3)];
        return Some(body.trim().to_string());
    }

    let html_prefixes = [
        "<!doctype html",
        "<html",
        "<body",
        "<main",
        "<article",
        "<section",
        "<table",
        "<div",
    ];
    if html_prefixes.iter().any(|prefix| lower.starts_with(prefix)) && trimmed.contains('>') {
        return Some(trimmed.to_string());
    }
    None
}

fn compact_for_card(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let head: String = trimmed.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{}...", head)
}

#[allow(dead_code)]
fn readback_questions_label(questions_json: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(questions_json).ok()?;
    let questions = value
        .as_array()?
        .iter()
        .filter_map(|item| item.as_str().map(str::trim))
        .filter(|item| !item.is_empty())
        .take(3)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if questions.is_empty() {
        None
    } else {
        Some(compact_for_card(&questions.join("; "), 180))
    }
}
