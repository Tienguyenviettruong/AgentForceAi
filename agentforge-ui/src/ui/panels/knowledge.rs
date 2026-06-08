use crate::ui::text::TextView;
use gpui::prelude::FluentBuilder;
use gpui::EventEmitter;
use gpui::{
    canvas, div, px, App, AppContext, Context, Entity, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window,
};
use gpui_component::dock::PanelEvent;
use gpui_component::dock::{Panel, TitleStyle};
use gpui_component::{scroll::ScrollableElement as _, v_flex, ActiveTheme as _, Icon, IconName};
use std::collections::HashSet;
use std::sync::Arc;
use urlencoding::encode;

pub struct KnowledgePanel {
    focus_handle: gpui::FocusHandle,
    knowledge_service: Arc<crate::application::services::knowledge_service::KnowledgeService>,
    vault_path: Entity<String>,
    #[allow(dead_code)]
    obsidian_watcher: Arc<std::sync::Mutex<Option<notify::RecommendedWatcher>>>,
    all_items: Vec<crate::knowledge::core::KnowledgeItem>,
    items: Vec<crate::knowledge::core::KnowledgeItem>,
    active_filter: KnowledgeFilter,
    selected_item: Option<crate::knowledge::core::KnowledgeItem>,
    graph_pan: gpui::Point<f32>,
    graph_zoom: f32,
    hovered_node: Option<usize>,
    selected_node_idx: Option<usize>,
    dragging_node: Option<usize>,
    is_panning_graph: bool,
    is_panning_minimap: bool,
    last_mouse_pos: Option<gpui::Point<f32>>,
    node_positions: Vec<gpui::Point<f32>>,
    node_velocities: Vec<gpui::Point<f32>>,
    expanded_dirs: HashSet<String>,
}

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KnowledgeFilter {
    All,
    Documents,
    Memories,
    Artifacts,
}

#[derive(Debug, Default)]
struct TreeNode {
    name: String,
    is_file: bool,
    item_id: Option<uuid::Uuid>,
    children: BTreeMap<String, TreeNode>,
}

impl TreeNode {
    fn render(
        &self,
        panel: &KnowledgePanel,
        depth: usize,
        current_path: &str,
        cx: &Context<KnowledgePanel>,
    ) -> gpui::AnyElement {
        use gpui::IntoElement;

        let mut container = gpui::div().flex_col();

        if self.is_file {
            if let Some(id) = self.item_id {
                container =
                    container.child(panel.render_tree_file(self.name.clone(), id, depth, cx));
            }
        } else {
            if depth > 0 {
                let is_expanded = panel.is_dir_expanded(current_path);
                container = container.child(panel.render_tree_item(
                    IconName::Folder,
                    self.name.clone(),
                    is_expanded,
                    depth,
                    current_path.to_string(),
                    cx,
                ));
                if !is_expanded {
                    return container.into_any_element();
                }
            }
        }

        // Render children
        let next_depth = if self.is_file { depth } else { depth + 1 };
        for child in self.children.values() {
            let child_path = if current_path.is_empty() {
                child.name.clone()
            } else {
                format!("{}/{}", current_path, child.name)
            };
            container = container.child(child.render(panel, next_depth, &child_path, cx));
        }

        container.into_any_element()
    }
}

impl KnowledgePanel {
    fn preprocess_obsidian_markdown(input: &str) -> String {
        let normalized = input
            .replace("0\u{fe0f}\u{20e3}", "0.")
            .replace("1\u{fe0f}\u{20e3}", "1.")
            .replace("2\u{fe0f}\u{20e3}", "2.")
            .replace("3\u{fe0f}\u{20e3}", "3.")
            .replace("4\u{fe0f}\u{20e3}", "4.")
            .replace("5\u{fe0f}\u{20e3}", "5.")
            .replace("6\u{fe0f}\u{20e3}", "6.")
            .replace("7\u{fe0f}\u{20e3}", "7.")
            .replace("8\u{fe0f}\u{20e3}", "8.")
            .replace("9\u{fe0f}\u{20e3}", "9.");
        let mut out = String::new();
        let mut in_code = false;
        for line in normalized.replace('\r', "").split('\n') {
            if line.trim_start().starts_with("```") {
                in_code = !in_code;
                out.push_str(line);
                out.push('\n');
                continue;
            }
            if in_code {
                out.push_str(line);
                out.push('\n');
                continue;
            }

            let mut processed = String::new();
            let mut i = 0usize;
            while let Some(start) = line[i..].find("[[") {
                let abs_start = i + start;
                processed.push_str(&line[i..abs_start]);
                if let Some(end) = line[abs_start + 2..].find("]]") {
                    let abs_end = abs_start + 2 + end;
                    let inner = &line[abs_start + 2..abs_end];
                    let (target, label) = inner
                        .split_once('|')
                        .map(|(a, b)| (a.trim(), b.trim()))
                        .unwrap_or((inner.trim(), inner.trim()));
                    let url = format!("obsidian://open?file={}", encode(target));
                    processed.push_str(&format!("[{}]({})", label, url));
                    i = abs_end + 2;
                } else {
                    processed.push_str("[[");
                    i = abs_start + 2;
                }
            }
            processed.push_str(&line[i..]);

            let mut tagged = String::new();
            let mut chars = processed.chars().peekable();
            let mut prev_ws = true;
            while let Some(ch) = chars.next() {
                if ch == '#' && prev_ws {
                    let mut tag = String::new();
                    while let Some(&c2) = chars.peek() {
                        if c2.is_ascii_alphanumeric() || c2 == '-' || c2 == '_' {
                            tag.push(c2);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if !tag.is_empty() {
                        let url = format!("agentforge://tag/{}", encode(&tag));
                        tagged.push_str(&format!("[#{}]({})", tag, url));
                        prev_ws = false;
                        continue;
                    }
                    tagged.push('#');
                    prev_ws = false;
                    continue;
                }
                prev_ws = ch.is_whitespace();
                tagged.push(ch);
            }

            out.push_str(&tagged);
            out.push('\n');
        }
        out
    }
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let db = crate::AppState::global(cx).db.clone();
        let knowledge_service = crate::AppState::global(cx).knowledge_service.clone();
        let tokio_runtime = crate::AppState::global(cx).tokio_runtime.clone();

        // Load initial vault path from db
        let initial_vault = db
            .get_setting("obsidian_vault_path")
            .ok()
            .flatten()
            .unwrap_or_default();
        let vault_path = cx.new(|_| initial_vault.clone());

        let mut items = Vec::new();
        if let Ok(db_items) = knowledge_service.get_all_records() {
            items = db_items;
        }

        let obsidian_watcher = Arc::new(std::sync::Mutex::new(None));

        if !initial_vault.is_empty() {
            if let Ok(watcher) =
                crate::infrastructure::fs::obsidian_adapter::ObsidianWatcher::start_sync(
                    db.clone(),
                    std::path::PathBuf::from(initial_vault),
                    tokio_runtime.clone(),
                )
            {
                *obsidian_watcher.lock().unwrap() = Some(watcher);
            }
        }

        // Auto-refresh background task
        cx.spawn(async move |view, cx| loop {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(3))
                .await;
            if crate::ui::framework::reentrancy::office_webview_init_in_progress() {
                continue;
            }
            if cx
                .update(|cx| {
                    let _ = view.update(cx, |this: &mut Self, cx| {
                        this.reload_items(cx);
                    });
                })
                .is_err()
            {
                break;
            }
        })
        .detach();

        // Physics tick
        cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(16))
                    .await;
                if crate::ui::framework::reentrancy::office_webview_init_in_progress() {
                    continue;
                }
                if cx
                    .update(|cx| {
                        let _ = view.update(cx, |this: &mut Self, cx| {
                            if this.node_positions.is_empty() {
                                return;
                            }

                            let n = this.items.len();
                            if this.node_positions.len() != n || this.node_velocities.len() != n {
                                return; // Length mismatch, wait for render to re-initialize
                            }
                            let mut edges = Vec::new();
                            for (i, item) in this.items.iter().enumerate() {
                                for (j, other_item) in this.items.iter().enumerate() {
                                    if Self::items_are_related_for_graph(i, item, j, other_item) {
                                        edges.push((i, j));
                                    }
                                }
                            }

                            let k = 100.0; // Optimal distance
                            let c = 0.1; // Repulsion constant
                            let dt = 0.05; // Time step
                            let damping = 0.85; // Damping

                            for i in 0..n {
                                let mut fx = 0.0;
                                let mut fy = 0.0;

                                let p1 = this.node_positions[i];

                                // Repulsion from all other nodes
                                for j in 0..n {
                                    if i == j {
                                        continue;
                                    }
                                    let p2 = this.node_positions[j];
                                    let dx = p1.x - p2.x;
                                    let dy = p1.y - p2.y;
                                    let dist_sq = dx * dx + dy * dy;
                                    let dist = dist_sq.sqrt().max(1.0);

                                    let force = c * (k * k) / dist;
                                    fx += force * (dx / dist);
                                    fy += force * (dy / dist);
                                }

                                // Attraction to center
                                fx -= p1.x * 0.05;
                                fy -= p1.y * 0.05;

                                this.node_velocities[i].x += fx * dt;
                                this.node_velocities[i].y += fy * dt;
                            }

                            // Attraction along edges
                            for (i, j) in edges {
                                let p1 = this.node_positions[i];
                                let p2 = this.node_positions[j];
                                let dx = p2.x - p1.x;
                                let dy = p2.y - p1.y;
                                let dist = (dx * dx + dy * dy).sqrt().max(1.0);

                                let force = (dist * dist) / k;
                                let fx = force * (dx / dist) * 0.05;
                                let fy = force * (dy / dist) * 0.05;

                                this.node_velocities[i].x += fx * dt;
                                this.node_velocities[i].y += fy * dt;
                                this.node_velocities[j].x -= fx * dt;
                                this.node_velocities[j].y -= fy * dt;
                            }

                            let mut moved = false;
                            for i in 0..n {
                                this.node_velocities[i].x *= damping;
                                this.node_velocities[i].y *= damping;

                                if this.node_velocities[i].x.abs() > 0.1
                                    || this.node_velocities[i].y.abs() > 0.1
                                {
                                    moved = true;
                                }

                                this.node_positions[i].x += this.node_velocities[i].x * dt;
                                this.node_positions[i].y += this.node_velocities[i].y * dt;
                            }

                            if moved {
                                cx.notify();
                            }
                        });
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

        Self {
            focus_handle: cx.focus_handle(),
            knowledge_service,
            vault_path,
            obsidian_watcher,
            all_items: items.clone(),
            items,
            active_filter: KnowledgeFilter::All,
            selected_item: None,
            graph_pan: gpui::point(0.0, 0.0),
            graph_zoom: 1.0,
            hovered_node: None,
            selected_node_idx: None,
            dragging_node: None,
            is_panning_graph: false,
            is_panning_minimap: false,
            last_mouse_pos: None,
            node_positions: Vec::new(),
            node_velocities: Vec::new(),
            expanded_dirs: {
                let mut s = HashSet::new();
                s.insert(String::new());
                s
            },
        }
    }

    pub fn reload_items(&mut self, cx: &mut Context<Self>) {
        if let Ok(items) = self.knowledge_service.get_all_records() {
            self.all_items = items;
            self.apply_filter();
            cx.notify();
        }
    }

    fn apply_filter(&mut self) {
        self.items = self
            .all_items
            .iter()
            .filter(|item| match self.active_filter {
                KnowledgeFilter::All => true,
                KnowledgeFilter::Documents => {
                    item.record_kind == crate::knowledge::core::KnowledgeRecordKind::Document
                }
                KnowledgeFilter::Memories => {
                    item.record_kind == crate::knowledge::core::KnowledgeRecordKind::Memory
                }
                KnowledgeFilter::Artifacts => {
                    item.record_kind == crate::knowledge::core::KnowledgeRecordKind::Artifact
                }
            })
            .cloned()
            .collect();
    }

    fn render_filter_button(
        &self,
        label: &'static str,
        filter: KnowledgeFilter,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let selected = self.active_filter == filter;
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .text_xs()
            .text_color(if selected {
                theme.foreground
            } else {
                theme.muted_foreground
            })
            .bg(if selected {
                theme.secondary
            } else {
                gpui::transparent_black()
            })
            .cursor_pointer()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.active_filter = filter;
                    this.selected_item = None;
                    this.selected_node_idx = None;
                    this.apply_filter();
                    cx.notify();
                }),
            )
            .child(label)
    }

    fn render_tree_item(
        &self,
        icon: IconName,
        label: String,
        is_expanded: bool,
        depth: usize,
        dir_key: String,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .pl(px(8.0 + (depth as f32) * 16.0))
            .rounded_md()
            .hover(|s| s.bg(theme.secondary))
            .cursor_pointer()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _event, _window, cx| {
                    if this.expanded_dirs.contains(&dir_key) {
                        this.expanded_dirs.remove(&dir_key);
                    } else {
                        this.expanded_dirs.insert(dir_key.clone());
                    }
                    cx.notify();
                }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .text_color(theme.muted_foreground)
                    .child(
                        Icon::new(if is_expanded {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .size(px(14.)),
                    ),
            )
            .child(Icon::new(icon).size(px(16.)).text_color(theme.accent))
            .child(div().text_sm().text_color(theme.foreground).child(label))
    }

    fn is_dir_expanded(&self, dir_key: &str) -> bool {
        self.expanded_dirs.contains(dir_key)
    }

    fn instance_display_name(&self, instance_id: &str, cx: &Context<Self>) -> String {
        crate::AppState::global(cx)
            .db
            .list_instances()
            .unwrap_or_default()
            .into_iter()
            .find(|instance| instance.id == instance_id)
            .map(|instance| instance.name)
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| instance_id.to_string())
    }

    fn agent_display_name(&self, agent_id: &str, cx: &Context<Self>) -> String {
        crate::AppState::global(cx)
            .db
            .get_agent(agent_id)
            .ok()
            .flatten()
            .map(|agent| agent.name)
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| Self::short_id(agent_id))
    }

    fn short_id(value: &str) -> String {
        let trimmed = value.trim();
        if trimmed.len() <= 12 {
            trimmed.to_string()
        } else {
            trimmed.chars().take(8).collect()
        }
    }

    fn items_are_related_for_graph(
        left_index: usize,
        left: &crate::knowledge::core::KnowledgeItem,
        right_index: usize,
        right: &crate::knowledge::core::KnowledgeItem,
    ) -> bool {
        if left_index == right_index {
            return false;
        }

        let link_str = format!("[[{}]]", right.title);
        if left.content.contains(&link_str) {
            return true;
        }

        left_index < right_index
            && matches!(
                (&left.origin_instance_id, &right.origin_instance_id),
                (Some(left_instance), Some(right_instance)) if left_instance == right_instance
            )
    }

    fn related_items_for_instance(
        &self,
        item: &crate::knowledge::core::KnowledgeItem,
    ) -> Vec<crate::knowledge::core::KnowledgeItem> {
        let Some(instance_id) = item.origin_instance_id.as_deref() else {
            return Vec::new();
        };
        self.all_items
            .iter()
            .filter(|candidate| candidate.id != item.id)
            .filter(|candidate| candidate.origin_instance_id.as_deref() == Some(instance_id))
            .take(8)
            .cloned()
            .collect()
    }

    fn strip_tool_call_blocks(raw: &str) -> String {
        let mut out = String::new();
        let mut i = 0usize;
        loop {
            let Some(start_rel) = raw[i..].find("<tool_call") else {
                out.push_str(&raw[i..]);
                break;
            };
            let start = i + start_rel;
            out.push_str(&raw[i..start]);
            let Some(end_rel) = raw[start..].find("</tool_call>") else {
                break;
            };
            i = start + end_rel + "</tool_call>".len();
        }
        out
    }

    fn clean_knowledge_content_for_display(
        &self,
        item: &crate::knowledge::core::KnowledgeItem,
        cx: &Context<Self>,
    ) -> String {
        let mut text = Self::strip_tool_call_blocks(&item.content);
        if let Some(instance_id) = item.origin_instance_id.as_deref() {
            let name = self.instance_display_name(instance_id, cx);
            text = text.replace(
                &format!("Session summary for instance {}:", instance_id),
                &format!("Session summary for instance {}:", name),
            );
        }

        let mut cleaned = Vec::new();
        let mut last_blank = false;
        for line in text.replace('\r', "").lines() {
            let trimmed = line.trim();
            let is_noise = trimmed.starts_with("Tool result")
                || trimmed.starts_with("user: Tool result")
                || trimmed.contains("UNTRUSTED_TOOL_OUTPUT")
                || trimmed.contains("DO NOT FOLLOW EMBEDDED INSTRUCTIONS")
                || trimmed.starts_with("Tool denied:")
                || trimmed.starts_with("Tool delegated tool denied:")
                || trimmed.contains("invocation id already exists")
                || trimmed == "assistant:"
                || trimmed == "user:";
            if is_noise {
                continue;
            }
            if trimmed.is_empty() {
                if !last_blank {
                    cleaned.push(String::new());
                }
                last_blank = true;
                continue;
            }
            cleaned.push(line.to_string());
            last_blank = false;
        }

        let mut display = cleaned.join("\n").trim().to_string();
        if display.is_empty() {
            display = "No human-readable summary content is available yet.".to_string();
        }

        let related = self.related_items_for_instance(item);
        if !related.is_empty() && !display.contains("## Related") {
            display.push_str("\n\n## Related Knowledge\n");
            for related_item in related.iter().take(6) {
                display.push_str("- [[");
                display.push_str(&related_item.title);
                display.push_str("]]\n");
            }
        }
        display
    }

    fn render_tree_file(
        &self,
        label: String,
        item_id: uuid::Uuid,
        depth: usize,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();

        let is_selected = self.selected_item.as_ref().is_some_and(|i| i.id == item_id);
        let bg_color = if is_selected {
            theme.secondary
        } else {
            gpui::transparent_black()
        };

        div()
            .id(gpui::ElementId::Name(format!("file-{}", item_id).into()))
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .pl(px(8.0 + (depth as f32) * 16.0))
            .rounded_md()
            .bg(bg_color)
            .hover(|s| s.bg(theme.secondary))
            .cursor_pointer()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _event, _window, cx| {
                    if let Some(item) = this.items.iter().find(|i| i.id == item_id) {
                        this.selected_item = Some(item.clone());
                        this.selected_node_idx = this.items.iter().position(|i| i.id == item_id);
                        cx.notify();
                    }
                }),
            )
            .child(
                Icon::new(IconName::File)
                    .size(px(14.))
                    .text_color(theme.muted_foreground),
            )
            .child(div().text_sm().text_color(theme.foreground).child(label))
    }

    fn render_tree_navigation(&self, _window: &mut Window, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let vault_path_value = self.vault_path.read(cx).clone();
        let display_path = if vault_path_value.is_empty() {
            "No vault selected".to_string()
        } else {
            vault_path_value.to_string()
        };
        div()
            .w(px(250.))
            .h_full()
            .border_r_1()
            .border_color(theme.border)
            .flex_col()
            .child(
                div()
                    .p_4()
                    .border_b_1()
                    .border_color(theme.border)
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("Knowledge Sources"),
                    )
                    .child(div().text_xs().child(display_path))
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .mt_1()
                            .child(self.render_filter_button("All", KnowledgeFilter::All, cx))
                            .child(self.render_filter_button(
                                "Documents",
                                KnowledgeFilter::Documents,
                                cx,
                            ))
                            .child(self.render_filter_button(
                                "Memories",
                                KnowledgeFilter::Memories,
                                cx,
                            ))
                            .child(self.render_filter_button(
                                "Artifacts",
                                KnowledgeFilter::Artifacts,
                                cx,
                            )),
                    ),
            )
            .child({
                let mut file_list = div()
                    .flex_1()
                    .p_2()
                    .id("scroll-sheet")
                    .overflow_y_scroll()
                    .flex_col()
                    .gap_1()
                    .child(self.render_tree_item(
                        IconName::Folder,
                        "Knowledge".to_string(),
                        self.is_dir_expanded(""),
                        0,
                        "".to_string(),
                        cx,
                    ));

                if self.items.is_empty() {
                    file_list = file_list.child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .p_2()
                            .child("No documents synced yet."),
                    );
                } else {
                    let mut root = TreeNode::default();
                    root.name = "Knowledge".to_string();
                    let vault_path_value = self.vault_path.read(cx).clone();
                    let mut vault_root_path = std::path::PathBuf::from(&vault_path_value);
                    if let Ok(p) = vault_root_path.canonicalize() {
                        vault_root_path = p;
                    }

                    for item in &self.items {
                        let rel_path = if item.record_kind
                            == crate::knowledge::core::KnowledgeRecordKind::Memory
                        {
                            std::path::PathBuf::from("Memories").join(format!("{}.md", item.title))
                        } else if item.record_kind
                            == crate::knowledge::core::KnowledgeRecordKind::Artifact
                        {
                            std::path::PathBuf::from("Artifacts").join(item.title.clone())
                        } else {
                            let document_path = if let Some(abs_path_str) = &item.vault_path {
                                let mut abs_path = std::path::PathBuf::from(abs_path_str);
                                if let Ok(p) = abs_path.canonicalize() {
                                    abs_path = p;
                                }
                                if vault_path_value.is_empty() {
                                    abs_path
                                        .file_name()
                                        .map(std::path::PathBuf::from)
                                        .unwrap_or_default()
                                } else {
                                    match abs_path.strip_prefix(&vault_root_path) {
                                        Ok(p) => p.to_path_buf(),
                                        Err(_) => abs_path
                                            .file_name()
                                            .map(std::path::PathBuf::from)
                                            .unwrap_or_default(),
                                    }
                                }
                            } else {
                                std::path::PathBuf::from(format!("{}.md", item.title))
                            };
                            std::path::PathBuf::from("Documents").join(document_path)
                        };

                        let components: Vec<_> = rel_path
                            .components()
                            .map(|c| c.as_os_str().to_string_lossy().to_string())
                            .collect();

                        let mut current = &mut root;
                        for (i, comp) in components.iter().enumerate() {
                            let is_last = i == components.len() - 1;
                            let node =
                                current
                                    .children
                                    .entry(comp.clone())
                                    .or_insert_with(|| TreeNode {
                                        name: comp.clone(),
                                        is_file: is_last,
                                        item_id: if is_last { Some(item.id) } else { None },
                                        children: BTreeMap::new(),
                                    });
                            current = node;
                        }
                    }

                    for child in root.children.values() {
                        if self.is_dir_expanded("") {
                            let child_path = child.name.clone();
                            file_list = file_list.child(child.render(self, 1, &child_path, cx));
                        }
                    }
                }

                file_list
            })
    }

    fn render_graph_visualization(&mut self, cx: &Context<Self>) -> impl IntoElement {
        use gpui::{canvas, point};

        let theme = cx.theme();
        let border_color = theme.border;
        let base_node_color = gpui::hsla(0.0, 0.0, 0.5, 1.0); // Gray color
        let highlight_node_color = theme.accent;
        let text_color = theme.foreground;
        let muted_text_color = theme.muted_foreground;

        // Build graph data
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for item in &self.items {
            nodes.push((item.id, item.title.clone()));
        }

        let n = nodes.len();

        // Link explicit [[wiki-links]] and memories/artifacts from the same instance.
        for (i, item) in self.items.iter().enumerate() {
            for (j, other_item) in self.items.iter().enumerate() {
                if Self::items_are_related_for_graph(i, item, j, other_item) {
                    edges.push((i, j));
                }
            }
        }

        let pan = self.graph_pan;
        let zoom = self.graph_zoom;
        let hovered_idx = self.hovered_node;

        // Use physics positions
        if self.node_positions.len() != n {
            let mut pos = Vec::with_capacity(n);
            let mut vel = Vec::with_capacity(n);
            let radius_base = 200.0;
            for i in 0..n {
                let angle = (i as f32 / n.max(1) as f32) * std::f32::consts::PI * 2.0;
                pos.push(point(radius_base * angle.cos(), radius_base * angle.sin()));
                vel.push(point(0.0, 0.0));
            }
            self.node_positions = pos;
            self.node_velocities = vel;
            self.dragging_node = None;
            self.last_mouse_pos = None;
        }

        if let Some(i) = self.dragging_node {
            if i >= self.node_positions.len() {
                self.dragging_node = None;
                self.last_mouse_pos = None;
            }
        }

        if let Some(i) = self.hovered_node {
            if i >= self.node_positions.len() {
                self.hovered_node = None;
            }
        }

        if let Some(i) = self.selected_node_idx {
            if i >= self.node_positions.len() {
                self.selected_node_idx = None;
            }
        }

        let node_positions = self.node_positions.clone();

        // We want the graph centered.
        let center_offset_x = 400.0;
        let center_offset_y = 300.0;

        div()
            .flex_1()
            .h_full()
            .flex_col()
            .child(
                div()
                    .flex_1()
                    .bg(theme.background)
                    .w_full()
                    .h_full()
                    .overflow_hidden()
                    .relative()
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, e: &gpui::MouseDownEvent, _window, _cx| {
                        if this.dragging_node.is_none() && !this.is_panning_minimap {
                            this.is_panning_graph = true;
                            let current_pos = gpui::Point { x: e.position.x.into(), y: e.position.y.into() };
                            this.last_mouse_pos = Some(current_pos);
                        }
                    }))
                    .on_mouse_up(gpui::MouseButton::Left, cx.listener(|this, _, _window, cx| {
                        this.dragging_node = None;
                        this.is_panning_graph = false;
                        this.is_panning_minimap = false;
                        this.last_mouse_pos = None;
                        cx.notify();
                    }))
                    .on_mouse_move(cx.listener(|this, e: &gpui::MouseMoveEvent, _window, cx| {
                        let current_pos = gpui::Point { x: e.position.x.into(), y: e.position.y.into() };
                        if let Some(last_pos) = this.last_mouse_pos {
                            let dx = current_pos.x - last_pos.x;
                            let dy = current_pos.y - last_pos.y;

                            if let Some(node_idx) = this.dragging_node {
                                if node_idx < this.node_positions.len() {
                                    let zoom = this.graph_zoom;
                                    this.node_positions[node_idx].x += dx / zoom;
                                    this.node_positions[node_idx].y += dy / zoom;
                                    cx.notify();
                                } else {
                                    this.dragging_node = None;
                                }
                            } else if this.is_panning_minimap {
                                // Minimap scale is 0.05. Moving mouse by dx on minimap means
                                // moving viewport by dx. Since vp_x = ... - pan.x * scale,
                                // to move vp_x by dx we need pan.x -= dx / scale.
                                this.graph_pan.x -= dx / 0.05;
                                this.graph_pan.y -= dy / 0.05;
                                cx.notify();
                            } else if this.is_panning_graph {
                                this.graph_pan.x += dx;
                                this.graph_pan.y += dy;
                                cx.notify();
                            }
                        }
                        this.last_mouse_pos = Some(current_pos);
                    }))
                    .on_scroll_wheel(cx.listener(|this, event: &gpui::ScrollWheelEvent, _window, cx| {
                        let old_zoom = this.graph_zoom;

                        let delta = match event.delta {
                            gpui::ScrollDelta::Pixels(p) => {
                                let py: f32 = p.y.into();
                                py
                            },
                            gpui::ScrollDelta::Lines(l) => l.y * 20.0,
                        };

                        let new_zoom = (this.graph_zoom + (delta / 500.0)).clamp(0.1, 5.0);
                        this.graph_zoom = new_zoom;

                        let mouse_x: f32 = event.position.x.into();
                        let mouse_y: f32 = event.position.y.into();
                        let zoom_ratio = new_zoom / old_zoom;

                        this.graph_pan.x = mouse_x - (mouse_x - this.graph_pan.x) * zoom_ratio;
                        this.graph_pan.y = mouse_y - (mouse_y - this.graph_pan.y) * zoom_ratio;

                        cx.notify();
                    }))
                    .child({
                        let edges_for_canvas = edges.clone();
                        let node_positions_for_canvas = node_positions.clone();
                        let accent = highlight_node_color;
                        let border = border_color;
                        let selected_idx = self.selected_node_idx;
                        canvas(
                            move |_bounds, _window, _cx| {},
                            move |bounds, _, window, _cx| {
                                for (from, to) in &edges_for_canvas {
                                    let p1_rel = node_positions_for_canvas[*from];
                                    let p2_rel = node_positions_for_canvas[*to];

                                    let start_x = gpui::px((p1_rel.x + center_offset_x) * zoom + pan.x) + bounds.origin.x;
                                    let start_y = gpui::px((p1_rel.y + center_offset_y) * zoom + pan.y) + bounds.origin.y;
                                    let end_x = gpui::px((p2_rel.x + center_offset_x) * zoom + pan.x) + bounds.origin.x;
                                    let end_y = gpui::px((p2_rel.y + center_offset_y) * zoom + pan.y) + bounds.origin.y;

                                    let is_highlighted = hovered_idx == Some(*from) || hovered_idx == Some(*to) ||
                                                         selected_idx == Some(*from) || selected_idx == Some(*to);
                                    let edge_color = if is_highlighted { accent } else { border };
                                    let edge_width = if is_highlighted { 2.0 } else { 1.0 };

                                    let mut builder = gpui::PathBuilder::stroke(gpui::px(edge_width)).with_style(
                                        gpui::PathStyle::Stroke(gpui::StrokeOptions::default()),
                                    );
                                    builder.move_to(gpui::point(start_x, start_y));
                                    builder.line_to(gpui::point(end_x, end_y));
                                    if let Ok(path) = builder.build() {
                                        window.paint_path(path, edge_color);
                                    }
                                }
                            }
                        ).size_full().absolute().top_0().left_0()
                    })
                    .children({
                        let mut nodes_ui = Vec::new();
                        for i in 0..n {
                            let title = nodes[i].1.clone();
                            let item_id = nodes[i].0;
                            let p_rel = node_positions[i];
                            let is_hovered = hovered_idx == Some(i);
                            let is_selected = self.selected_node_idx == Some(i);

                            // Highlight node if hovered, selected, or connected to hovered/selected
                            let mut is_highlighted = is_hovered || is_selected;
                            if !is_highlighted {
                                for (from, to) in &edges {
                                    if (*from == i && (hovered_idx == Some(*to) || self.selected_node_idx == Some(*to))) ||
                                       (*to == i && (hovered_idx == Some(*from) || self.selected_node_idx == Some(*from))) {
                                        is_highlighted = true;
                                        break;
                                    }
                                }
                            }

                            let node_size = if is_hovered || is_selected { 12.0 } else { 8.0 };
                            let color = if is_highlighted { highlight_node_color } else { base_node_color };
                            let current_text_color = if is_highlighted { text_color } else { muted_text_color };

                            // Base center point in unzoomed coords
                            let center_x = p_rel.x + center_offset_x;
                            let center_y = p_rel.y + center_offset_y;

                            // Left and top relative to parent container (centered)
                            let left_pos = gpui::px(center_x * zoom + pan.x - node_size);
                            let top_pos = gpui::px(center_y * zoom + pan.y - node_size);

                            nodes_ui.push(
                                div()
                                    .absolute()
                                    .left(left_pos)
                                    .top(top_pos)
                                    .child(
                                        div()
                                            .w(px(node_size * 2.0))
                                            .h(px(node_size * 2.0))
                                            .rounded_full()
                                            .bg(color)
                                            .cursor_pointer()
                                            .hover(|s| s.bg(highlight_node_color))
                                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, e: &gpui::MouseDownEvent, _window, cx| {
                                                if e.click_count == 2 {
                                                    if let Some(item) = this.items.iter().find(|it| it.id == item_id) {
                                                        this.selected_item = Some(item.clone());
                                                        this.selected_node_idx = Some(i);
                                                        cx.notify();
                                                    }
                                                } else {
                                                    if let Some(item) = this.items.iter().find(|it| it.id == item_id) {
                                                        this.selected_item = Some(item.clone());
                                                    }
                                                    this.selected_node_idx = Some(i);
                                                    this.dragging_node = Some(i);
                                                    cx.notify();
                                                }
                                            }))
                                            // Optional hover logic to highlight edges
                                            // Unfortunately mouse enter/leave is tricky without explicit bounds.
                                            // We will just let hover state on the div handle color, but edge highlighting
                                            // would require MouseMove with bounds hit testing. Since we use GPUI components,
                                            // we can just use `on_mouse_down` or leave edges static.
                                            // Wait, `on_mouse_move` can capture the event.
                                    )
                                    .child(
                                        div()
                                            .absolute()
                                            .top(px(node_size * 2.0 + 4.0))
                                            // Move text left to center it relative to the node
                                            .left(px(node_size - 50.0))
                                            .w(px(100.0))
                                            .flex()
                                            .justify_center()
                                            .text_size(px(12.))
                                            .text_color(current_text_color)
                                            .child(title)
                                    )
                            );
                        }
                        nodes_ui
                    })
                    .child(self.render_minimap(cx))
            )
    }

    fn render_minimap(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let nodes = self.node_positions.clone();
        let pan = self.graph_pan;
        let zoom = self.graph_zoom;

        div()
            .absolute()
            .bottom_4()
            .right_4()
            .w(px(150.))
            .h(px(100.))
            .bg(theme.secondary.opacity(0.8))
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .overflow_hidden()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, e: &gpui::MouseDownEvent, _window, cx| {
                    this.is_panning_minimap = true;
                    let current_pos = gpui::Point {
                        x: e.position.x.into(),
                        y: e.position.y.into(),
                    };
                    this.last_mouse_pos = Some(current_pos);
                    cx.notify();
                }),
            )
            .child(
                canvas(
                    move |_bounds, _window, _cx| {},
                    move |bounds, _, window: &mut Window, _cx| {
                        let cx_offset = bounds.size.width / 2.0;
                        let cy_offset = bounds.size.height / 2.0;

                        // Minimap scaling factor
                        let scale = 0.05;

                        // Draw nodes
                        for pos in &nodes {
                            let start_x = bounds.origin.x + cx_offset + gpui::px(pos.x * scale);
                            let start_y = bounds.origin.y + cy_offset + gpui::px(pos.y * scale);

                            let rect = gpui::Bounds {
                                origin: gpui::point(
                                    start_x - gpui::px(2.0),
                                    start_y - gpui::px(2.0),
                                ),
                                size: gpui::size(gpui::px(4.0), gpui::px(4.0)),
                            };
                            window.paint_quad(gpui::fill(rect, gpui::rgba(0x00d4aaff)));
                        }

                        // Viewport rectangle
                        let vp_w = bounds.size.width / zoom;
                        let vp_h = bounds.size.height / zoom;
                        let vp_x =
                            bounds.origin.x + cx_offset - gpui::px(pan.x * scale) - vp_w / 2.0;
                        let vp_y =
                            bounds.origin.y + cy_offset - gpui::px(pan.y * scale) - vp_h / 2.0;

                        let mut builder = gpui::PathBuilder::stroke(gpui::px(1.0))
                            .with_style(gpui::PathStyle::Stroke(gpui::StrokeOptions::default()));
                        builder.move_to(gpui::point(vp_x, vp_y));
                        builder.line_to(gpui::point(vp_x + vp_w, vp_y));
                        builder.line_to(gpui::point(vp_x + vp_w, vp_y + vp_h));
                        builder.line_to(gpui::point(vp_x, vp_y + vp_h));
                        builder.line_to(gpui::point(vp_x, vp_y));

                        if let Ok(path) = builder.build() {
                            window.paint_path(path, gpui::rgba(0xffffff44));
                        }
                    },
                )
                .size_full(),
            )
    }

    fn render_stat_card(
        &self,
        icon: IconName,
        label: String,
        value: String,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .p_3()
            .bg(theme.secondary)
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .flex()
            .items_center()
            .gap_4()
            .child(
                div()
                    .p_2()
                    .bg(theme.background)
                    .rounded_md()
                    .child(Icon::new(icon).size(px(20.)).text_color(theme.accent)),
            )
            .child(
                div()
                    .flex_col()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(label),
                    )
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme.foreground)
                            .child(value),
                    ),
            )
    }

    fn render_analytics_dashboard(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let document_count = self
            .all_items
            .iter()
            .filter(|item| {
                item.record_kind == crate::knowledge::core::KnowledgeRecordKind::Document
            })
            .count();
        let memory_count = self
            .all_items
            .iter()
            .filter(|item| item.record_kind == crate::knowledge::core::KnowledgeRecordKind::Memory)
            .count();
        let artifact_count = self
            .all_items
            .iter()
            .filter(|item| {
                item.record_kind == crate::knowledge::core::KnowledgeRecordKind::Artifact
            })
            .count();
        let tag_count = self
            .all_items
            .iter()
            .flat_map(|item| item.tags.iter().map(|tag| tag.0.clone()))
            .collect::<HashSet<_>>()
            .len();
        let connection_count = self
            .all_items
            .iter()
            .map(|item| {
                self.all_items
                    .iter()
                    .filter(|other| {
                        item.id != other.id
                            && item.content.contains(&format!("[[{}]]", other.title))
                    })
                    .count()
            })
            .sum::<usize>();

        let mut recent_activity = v_flex().mt_4().gap(px(12.)).child(
            div()
                .text_sm()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.muted_foreground)
                .child("RECENT ACTIVITY"),
        );
        if self.all_items.is_empty() {
            recent_activity = recent_activity.child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("No knowledge saved yet."),
            );
        } else {
            for item in self.all_items.iter().take(4) {
                let kind = match item.record_kind {
                    crate::knowledge::core::KnowledgeRecordKind::Document => "Document",
                    crate::knowledge::core::KnowledgeRecordKind::Memory => "Memory",
                    crate::knowledge::core::KnowledgeRecordKind::Artifact => "Artifact",
                };
                recent_activity = recent_activity.child(
                    div()
                        .p_3()
                        .bg(theme.secondary)
                        .rounded_lg()
                        .border_1()
                        .border_color(theme.border)
                        .flex_col()
                        .gap_2()
                        .child(div().text_sm().child(item.title.clone()))
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(format!("{} / {}", kind, item.source_kind)),
                        ),
                );
            }
        }
        div()
            .w(px(300.))
            .h_full()
            .min_h_0()
            .border_l_1()
            .border_color(theme.border)
            .flex_col()
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .p_4()
                    .id("scroll-sheet")
                    .overflow_y_scrollbar()
                    .child(
                        v_flex()
                            .gap(px(12.))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.muted_foreground)
                                    .child("OVERVIEW"),
                            )
                            .child(self.render_stat_card(
                                IconName::File,
                                "Documents".to_string(),
                                format!("{}", document_count),
                                cx,
                            ))
                            .child(self.render_stat_card(
                                IconName::Info,
                                "Memories".to_string(),
                                format!("{}", memory_count),
                                cx,
                            ))
                            .child(self.render_stat_card(
                                IconName::File,
                                "Artifacts".to_string(),
                                format!("{}", artifact_count),
                                cx,
                            ))
                            .child(self.render_stat_card(
                                IconName::Info,
                                "Connections".to_string(),
                                format!("{}", connection_count),
                                cx,
                            ))
                            .child(self.render_stat_card(
                                IconName::Info,
                                "Tags Used".to_string(),
                                format!("{}", tag_count),
                                cx,
                            ))
                            .child(recent_activity),
                    ),
            )
    }

    fn render_markdown_viewer(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme();

        let Some(item) = self.selected_item.as_ref() else {
            return div().into_any_element();
        };

        let item_id = item.id;
        let title = item.title.clone();
        let kind = match item.record_kind {
            crate::knowledge::core::KnowledgeRecordKind::Document => "Document",
            crate::knowledge::core::KnowledgeRecordKind::Memory => "Memory",
            crate::knowledge::core::KnowledgeRecordKind::Artifact => "Artifact",
        };
        let mut provenance = vec![format!("{} / {}", kind, item.source_kind)];
        if let Some(instance_id) = &item.origin_instance_id {
            provenance.push(format!(
                "Instance {}",
                self.instance_display_name(instance_id, cx)
            ));
        }
        if let Some(run_id) = &item.origin_run_id {
            provenance.push(format!("Run {}", Self::short_id(run_id)));
        }
        if let Some(session_id) = &item.origin_session_id {
            provenance.push(format!("Session {}", Self::short_id(session_id)));
        }
        if let Some(agent_id) = &item.origin_agent_id {
            provenance.push(format!("Agent {}", self.agent_display_name(agent_id, cx)));
        }
        let related_items = self.related_items_for_instance(item);
        let content_text = gpui::SharedString::from(Self::preprocess_obsidian_markdown(
            &self.clean_knowledge_content_for_display(item, cx),
        ));

        div()
            .w(px(800.))
            .h_full()
            .border_l_1()
            .border_color(theme.border)
            .bg(theme.background)
            .flex_col()
            .min_w(px(0.))
            .overflow_hidden()
            .child(
                div()
                    .h(px(40.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .child(title),
                    )
                    .child(
                        div()
                            .p_1()
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.secondary))
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|this, _event, _window, cx| {
                                    this.selected_item = None;
                                    this.selected_node_idx = None;
                                    cx.notify();
                                }),
                            )
                            .child(
                                Icon::new(IconName::Close)
                                    .size(px(16.))
                                    .text_color(theme.muted_foreground),
                            ),
                    ),
            )
            .child(
                div()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .flex()
                    .gap_2()
                    .items_center()
                    .flex_wrap()
                    .children(provenance.into_iter().map(|value| {
                        div()
                            .px_2()
                            .py_1()
                            .max_w(px(220.))
                            .rounded_md()
                            .bg(theme.secondary)
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .truncate()
                            .child(value)
                    })),
            )
            .when(!related_items.is_empty(), |panel| {
                panel.child(
                    div()
                        .px_4()
                        .py_2()
                        .border_b_1()
                        .border_color(theme.border)
                        .flex()
                        .gap_2()
                        .items_center()
                        .flex_wrap()
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("Related"),
                        )
                        .children(related_items.into_iter().take(6).map(|related| {
                            let related_id = related.id;
                            div()
                                .px_2()
                                .py_1()
                                .max_w(px(220.))
                                .rounded_md()
                                .bg(theme.secondary)
                                .text_xs()
                                .text_color(theme.foreground)
                                .truncate()
                                .cursor_pointer()
                                .hover(|style| style.bg(theme.secondary_active))
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    cx.listener(move |this, _event, _window, cx| {
                                        if let Some(item) = this
                                            .all_items
                                            .iter()
                                            .find(|candidate| candidate.id == related_id)
                                            .cloned()
                                        {
                                            this.selected_item = Some(item);
                                            this.selected_node_idx = this
                                                .items
                                                .iter()
                                                .position(|candidate| candidate.id == related_id);
                                            cx.notify();
                                        }
                                    }),
                                )
                                .child(related.title)
                        })),
                )
            })
            .child(
                div()
                    .id(gpui::ElementId::Name(
                        format!("knowledge-markdown-scroll-{}", item_id).into(),
                    ))
                    .flex_1()
                    .min_h_0()
                    .min_w(px(0.))
                    .overflow_y_scrollbar()
                    .child(
                        TextView::markdown(
                            gpui::ElementId::Name(
                                format!("knowledge-markdown-preview-{}", item_id).into(),
                            ),
                            content_text,
                        )
                        .w_full()
                        .p_6()
                        .selectable(true),
                    ),
            )
            .into_any_element()
    }
}

impl Panel for KnowledgePanel {
    fn panel_name(&self) -> &'static str {
        "Knowledge"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.panel_name()
    }

    fn title_style(&self, _cx: &App) -> Option<TitleStyle> {
        None
    }
}

impl Focusable for KnowledgePanel {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for KnowledgePanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .flex()
            .flex_row()
            .child(self.render_tree_navigation(window, cx))
            .child(self.render_graph_visualization(cx))
            .child(if self.selected_item.is_some() {
                self.render_markdown_viewer(cx)
            } else {
                self.render_analytics_dashboard(cx).into_any_element()
            })
    }
}

impl EventEmitter<PanelEvent> for KnowledgePanel {}
