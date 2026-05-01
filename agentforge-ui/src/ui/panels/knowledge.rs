use gpui::EventEmitter;
use crate::core::traits::database::DatabasePort;
use gpui::{StatefulInteractiveElement, Entity, 
    canvas, div, px, App, AppContext, Context, Focusable, InteractiveElement, IntoElement, ParentElement, Render,
    Styled, Window,
};
use gpui_component::dock::PanelEvent;
use gpui_component::dock::{Panel, TitleStyle};
use gpui_component::{ActiveTheme as _, Icon, IconName, Placement, WindowExt as _, scroll::ScrollableElement, h_flex, v_flex};
use gpui_component::text::TextView;
use std::sync::Arc;
use urlencoding::encode;

pub struct KnowledgePanel {
    focus_handle: gpui::FocusHandle,
    db: Arc<dyn DatabasePort>,
    knowledge_service: Arc<crate::application::services::knowledge_service::KnowledgeService>,
    vault_path: Entity<String>,
    tokio_runtime: Arc<tokio::runtime::Runtime>,
    obsidian_watcher: Arc<std::sync::Mutex<Option<notify::RecommendedWatcher>>>,
    items: Vec<crate::knowledge::core::KnowledgeItem>,
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
}

use std::collections::BTreeMap;

#[derive(Debug, Default)]
struct TreeNode {
    name: String,
    is_file: bool,
    item_id: Option<uuid::Uuid>,
    children: BTreeMap<String, TreeNode>,
}

impl TreeNode {
    fn render(&self, panel: &KnowledgePanel, depth: usize, cx: &Context<KnowledgePanel>) -> gpui::AnyElement {
        use gpui::IntoElement;
        
        
        
        let mut container = gpui::div().flex_col();
        
        if self.is_file {
            if let Some(id) = self.item_id {
                container = container.child(panel.render_tree_file(self.name.clone(), id, depth, cx));
            }
        } else {
            // Don't render the root node itself as a folder item because we do it manually,
            // or we just render it here. If depth > 0, it's a real subfolder.
            if depth > 0 {
                container = container.child(panel.render_tree_item(IconName::Folder, self.name.clone(), true, depth, cx));
            }
        }
        
        // Render children
        let next_depth = if self.is_file { depth } else { depth + 1 };
        for child in self.children.values() {
            container = container.child(child.render(panel, next_depth, cx));
        }
        
        container.into_any_element()
    }
}

impl KnowledgePanel {
    fn preprocess_obsidian_markdown(input: &str) -> String {
        let mut out = String::new();
        let mut in_code = false;
        for line in input.replace('\r', "").split('\n') {
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
        if let Ok(db_items) = knowledge_service.get_all_knowledge_items() {
            items = db_items;
        }

        let obsidian_watcher = Arc::new(std::sync::Mutex::new(None));
        
        if !initial_vault.is_empty() {
            if let Ok(watcher) = crate::infrastructure::fs::obsidian_adapter::ObsidianWatcher::start_sync(
                db.clone(),
                std::path::PathBuf::from(initial_vault),
                tokio_runtime.clone(),
            ) {
                *obsidian_watcher.lock().unwrap() = Some(watcher);
            }
        }

        // Auto-refresh background task
        cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor().timer(std::time::Duration::from_secs(3)).await;
                if cx.update(|cx| {
                    let _ = view.update(cx, |this: &mut Self, cx| {
                        this.reload_items(cx);
                    });
                }).is_err() {
                    break;
                }
            }
        }).detach();

        // Physics tick
        cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor().timer(std::time::Duration::from_millis(16)).await;
                if cx.update(|cx| {
                    let _ = view.update(cx, |this: &mut Self, cx| {
                        if this.node_positions.is_empty() { return; }
                        
                        let n = this.items.len();
                        if this.node_positions.len() != n || this.node_velocities.len() != n {
                            return; // Length mismatch, wait for render to re-initialize
                        }
                        let mut edges = Vec::new();
                        for (i, item) in this.items.iter().enumerate() {
                            let content = &item.content;
                            for (j, other_item) in this.items.iter().enumerate() {
                                if i != j {
                                    let link_str = format!("[[{}]]", other_item.title);
                                    if content.contains(&link_str) {
                                        edges.push((i, j));
                                    }
                                }
                            }
                        }
                        
                        let k = 100.0; // Optimal distance
                        let c = 0.1;   // Repulsion constant
                        let dt = 0.05; // Time step
                        let damping = 0.85; // Damping
                        
                        for i in 0..n {
                            let mut fx = 0.0;
                            let mut fy = 0.0;
                            
                            let p1 = this.node_positions[i];
                            
                            // Repulsion from all other nodes
                            for j in 0..n {
                                if i == j { continue; }
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
                            
                            if this.node_velocities[i].x.abs() > 0.1 || this.node_velocities[i].y.abs() > 0.1 {
                                moved = true;
                            }
                            
                            this.node_positions[i].x += this.node_velocities[i].x * dt;
                            this.node_positions[i].y += this.node_velocities[i].y * dt;
                        }
                        
                        if moved {
                            cx.notify();
                        }
                    });
                }).is_err() {
                    break;
                }
            }
        }).detach();

        Self {
            focus_handle: cx.focus_handle(),
            db,
            knowledge_service,
            vault_path,
            tokio_runtime,
            obsidian_watcher,
            items,
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
        }
    }
    
    pub fn reload_items(&mut self, cx: &mut Context<Self>) {
        if let Ok(items) = self.knowledge_service.get_all_knowledge_items() {
            self.items = items;
            cx.notify();
        }
    }

    fn render_tree_item(
        &self,
        icon: IconName,
        label: String,
        is_expanded: bool,
        depth: usize,
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

    fn render_tree_file(&self, label: String, item_id: uuid::Uuid, depth: usize, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        
        let is_selected = self.selected_item.as_ref().is_some_and(|i| i.id == item_id);
        let bg_color = if is_selected { theme.secondary } else { gpui::transparent_black() };
        
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
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _event, window, cx| {
                if let Some(item) = this.items.iter().find(|i| i.id == item_id) {
                    this.selected_item = Some(item.clone());
                    
                    let title = item.title.clone();
                    let content_text = gpui::SharedString::from(Self::preprocess_obsidian_markdown(&item.content));
                    
                    window.open_sheet_at(Placement::Right, cx, move |sheet, window, cx| {
                        sheet.title(title.clone()).size(px(800.)).child(
                            gpui::div()
                                .flex_1()
                                .w_full()
                                .overflow_y_scrollbar()
                                .child(
                                    TextView::markdown(
                                        ("knowledge-sheet", 0usize),
                                        content_text.clone(),
                                        window,
                                        cx,
                                    )
                                    .p_6()
                                    .selectable(true),
                                )
                        )
                    });
                    
                    cx.notify();
                }
            }))
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
                    .child(div().text_xs().text_color(theme.muted_foreground).child("Obsidian Vault"))
                    .child(div().text_xs().child(display_path))
                    .child(div().text_xs().text_color(theme.muted_foreground).child("(Configure in Settings)"))
            )
            .child({
                let mut file_list = div()
                    .flex_1()
                    .p_2()
                    .id("scroll-sheet").overflow_y_scroll()
                    .flex_col()
                    .gap_1()
                    .child(self.render_tree_item(IconName::Folder, "Vault Root".to_string(), true, 0, cx));
                    
                if self.items.is_empty() {
                    file_list = file_list.child(
                        div().text_sm().text_color(theme.muted_foreground).p_2().child("No documents synced yet.")
                    );
                } else {
                    let mut root = TreeNode::default();
                    root.name = "Vault Root".to_string();
                    let vault_path_value = self.vault_path.read(cx).clone();
                    let vault_root_path = std::path::Path::new(&vault_path_value);
                    
                    for item in &self.items {
                        let rel_path = if let Some(abs_path_str) = &item.vault_path {
                            let abs_path = std::path::Path::new(abs_path_str);
                            match abs_path.strip_prefix(vault_root_path) {
                                Ok(p) => p.to_path_buf(),
                                Err(_) => std::path::PathBuf::from(abs_path.file_name().unwrap_or_default())
                            }
                        } else {
                            std::path::PathBuf::from(format!("{}.md", item.title))
                        };
                        
                        let components: Vec<_> = rel_path.components()
                            .map(|c| c.as_os_str().to_string_lossy().to_string())
                            .collect();
                            
                        let mut current = &mut root;
                        for (i, comp) in components.iter().enumerate() {
                            let is_last = i == components.len() - 1;
                            let node = current.children.entry(comp.clone()).or_insert_with(|| TreeNode {
                                name: comp.clone(),
                                is_file: is_last,
                                item_id: if is_last { Some(item.id) } else { None },
                                children: BTreeMap::new(),
                            });
                            current = node;
                        }
                    }
                    
                    for child in root.children.values() {
                        file_list = file_list.child(child.render(self, 1, cx));
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
        
        // Very basic link extraction [[Link]]
        for (i, item) in self.items.iter().enumerate() {
            let content = &item.content;
            for (j, other_item) in self.items.iter().enumerate() {
                if i != j {
                    let link_str = format!("[[{}]]", other_item.title);
                    if content.contains(&link_str) {
                        edges.push((i, j));
                    }
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
                                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, e: &gpui::MouseDownEvent, window, cx| {
                                                if e.click_count == 2 {
                                                    if let Some(item) = this.items.iter().find(|it| it.id == item_id) {
                                                        let sheet_title = item.title.clone();
                                                        let content_text = Self::preprocess_obsidian_markdown(&item.content);
                                                        window.open_sheet_at(Placement::Right, cx, move |sheet, window, cx| {
                                                            sheet.title(sheet_title.clone()).size(px(800.)).child(
                                                                div()
                                                                    .id(("knowledge-sheet", 0usize))
                                                                    .w_full()
                                                                    .overflow_y_scrollbar()
                                                                    .child(
                                                                        TextView::markdown(
                                                                            ("knowledge-sheet", 0usize),
                                                                            gpui::SharedString::from(content_text.clone()),
                                                                            window,
                                                                            cx,
                                                                        )
                                                                        .p_6()
                                                                        .selectable(true),
                                                                    )
                                                            )
                                                        });
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
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, e: &gpui::MouseDownEvent, _window, cx| {
                this.is_panning_minimap = true;
                let current_pos = gpui::Point { x: e.position.x.into(), y: e.position.y.into() };
                this.last_mouse_pos = Some(current_pos);
                cx.notify();
            }))
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
                                origin: gpui::point(start_x - gpui::px(2.0), start_y - gpui::px(2.0)),
                                size: gpui::size(gpui::px(4.0), gpui::px(4.0)),
                            };
                            window.paint_quad(gpui::fill(rect, gpui::rgba(0x00d4aaff)));
                        }

                        // Viewport rectangle
                        let vp_w = bounds.size.width / zoom;
                        let vp_h = bounds.size.height / zoom;
                        let vp_x = bounds.origin.x + cx_offset - gpui::px(pan.x * scale) - vp_w / 2.0;
                        let vp_y = bounds.origin.y + cy_offset - gpui::px(pan.y * scale) - vp_h / 2.0;

                        let mut builder = gpui::PathBuilder::stroke(gpui::px(1.0)).with_style(
                            gpui::PathStyle::Stroke(gpui::StrokeOptions::default()),
                        );
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
        div()
            .w(px(300.))
            .h_full()
            .border_l_1()
            .border_color(theme.border)
            .flex_col()
            .child(
                div()
                    .flex_1()
                    .p_4()
                    .id("scroll-sheet").overflow_y_scroll()
                    .child(
                        v_flex()
                            .gap(px(12.))
                            .child(div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD).text_color(theme.muted_foreground).child("OVERVIEW"))
                            .child(self.render_stat_card(IconName::File, "Total Documents".to_string(), format!("{}", self.items.len()), cx))
                            .child(self.render_stat_card(IconName::Info, "Connections".to_string(), "0".to_string(), cx))
                            .child(self.render_stat_card(IconName::Info, "Tags Used".to_string(), "0".to_string(), cx))
                            .child(self.render_stat_card(IconName::Info, "Orphan Nodes".to_string(), "0".to_string(), cx))
                    )
                    .child(
                        v_flex()
                            .mt_4()
                            .gap(px(12.))
                            .child(div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD).text_color(theme.muted_foreground).child("RECENT ACTIVITY"))
                            .child(
                                div()
                                    .p_3()
                                    .bg(theme.secondary)
                                    .rounded_lg()
                                    .border_1()
                                    .border_color(theme.border)
                                    .flex_col()
                                    .gap_2()
                                    .child(div().text_sm().child("Updated 'Vector Databases.md'"))
                                    .child(div().text_xs().text_color(theme.muted_foreground).child("2 hours ago"))
                            )
                            .child(
                                div()
                                    .p_3()
                                    .bg(theme.secondary)
                                    .rounded_lg()
                                    .border_1()
                                    .border_color(theme.border)
                                    .flex_col()
                                    .gap_2()
                                    .child(div().text_sm().child("Linked 'LLM Comparisons.md' to 'Project Specifications.md'"))
                                    .child(div().text_xs().text_color(theme.muted_foreground).child("5 hours ago"))
                            )
                    ),
            )
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
            .child(self.render_analytics_dashboard(cx))
    }
}

impl EventEmitter<PanelEvent> for KnowledgePanel {}
