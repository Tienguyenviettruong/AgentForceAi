use crate::core::models::memory_bank::*;
use crate::ui::text::TextView;
use gpui::prelude::FluentBuilder;
use gpui::{
    canvas, div, fill, img, point, px, quad, App, AppContext, BorderStyle, Bounds, ContentMask,
    Context, Corner, Edges, Entity, EventEmitter, Focusable, Hsla, InteractiveElement, IntoElement,
    MouseButton, ObjectFit, ParentElement, Pixels, Point, Render, SharedString,
    StatefulInteractiveElement, Styled, StyledImage, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::dock::{Panel, PanelEvent, TitleStyle};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::popover::Popover;
use gpui_component::scroll::ScrollableElement;
use gpui_component::{h_flex, v_flex, ActiveTheme as _, Icon, IconName, Selectable, Sizable};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const SCENE_WIDTH: f32 = 1100.0;
const SCENE_HEIGHT: f32 = 760.0;
const LANDMARK_RENDER_SCALE: f32 = 0.75;

#[derive(Clone, Copy, PartialEq, Eq)]
enum MemoryViewMode {
    Map,
    Graph,
    List,
}

#[derive(Clone, Copy)]
struct DistrictLayout {
    category: MemoryBankCategory,
    asset_path: &'static str,
    cx: f32,
    cy: f32,
    asset_x: f32,
    asset_y: f32,
    asset_w: f32,
    asset_h: f32,
}

const DISTRICTS: [DistrictLayout; 6] = [
    DistrictLayout {
        category: MemoryBankCategory::SystemArchitecture,
        asset_path: "memory-bank/architecture.png",
        cx: 550.0,
        cy: 150.0,
        asset_x: 410.0,
        asset_y: 0.0,
        asset_w: 280.0,
        asset_h: 356.0,
    },
    DistrictLayout {
        category: MemoryBankCategory::ProjectBrief,
        asset_path: "memory-bank/project-brief.png",
        cx: 245.0,
        cy: 225.0,
        asset_x: 75.0,
        asset_y: 70.0,
        asset_w: 340.0,
        asset_h: 337.0,
    },
    DistrictLayout {
        category: MemoryBankCategory::ActiveContext,
        asset_path: "memory-bank/context.png",
        cx: 855.0,
        cy: 235.0,
        asset_x: 705.0,
        asset_y: 80.0,
        asset_w: 310.0,
        asset_h: 351.0,
    },
    DistrictLayout {
        category: MemoryBankCategory::Progress,
        asset_path: "memory-bank/progress.png",
        cx: 225.0,
        cy: 505.0,
        asset_x: 50.0,
        asset_y: 350.0,
        asset_w: 350.0,
        asset_h: 344.0,
    },
    DistrictLayout {
        category: MemoryBankCategory::LessonsLearned,
        asset_path: "memory-bank/lessons.png",
        cx: 490.0,
        cy: 620.0,
        asset_x: 320.0,
        asset_y: 440.0,
        asset_w: 330.0,
        asset_h: 331.0,
    },
    DistrictLayout {
        category: MemoryBankCategory::DecisionLog,
        asset_path: "memory-bank/decisions.png",
        cx: 835.0,
        cy: 550.0,
        asset_x: 675.0,
        asset_y: 395.0,
        asset_w: 335.0,
        asset_h: 335.0,
    },
];

const CORE_ASSET_PATH: &str = "memory-bank/project-core.png";
const CORE_ASSET_RECT: (f32, f32, f32, f32) = (420.0, 250.0, 260.0, 250.0);

#[derive(Clone, Copy)]
struct BridgeAssetLayout {
    asset_path: &'static str,
    category: MemoryBankCategory,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

const BRIDGE_ASSETS: [BridgeAssetLayout; 4] = [
    BridgeAssetLayout {
        asset_path: "memory-bank/data-bridge-nw.png",
        category: MemoryBankCategory::ProjectBrief,
        x: 300.0,
        y: 220.0,
        width: 250.0,
        height: 206.0,
    },
    BridgeAssetLayout {
        asset_path: "memory-bank/data-bridge-ne.png",
        category: MemoryBankCategory::ActiveContext,
        x: 555.0,
        y: 230.0,
        width: 270.0,
        height: 222.0,
    },
    BridgeAssetLayout {
        asset_path: "memory-bank/data-bridge-ne.png",
        category: MemoryBankCategory::Progress,
        x: 310.0,
        y: 370.0,
        width: 245.0,
        height: 202.0,
    },
    BridgeAssetLayout {
        asset_path: "memory-bank/data-bridge-nw.png",
        category: MemoryBankCategory::DecisionLog,
        x: 550.0,
        y: 380.0,
        width: 260.0,
        height: 214.0,
    },
];

const SENTINEL_ASSET_PATH: &str = "memory-bank/memory-sentinel.png";
const SENTINEL_RECTS: [(f32, f32, f32, f32); 3] = [
    (385.0, 312.0, 30.0, 38.0),
    (690.0, 340.0, 30.0, 38.0),
    (625.0, 560.0, 28.0, 35.0),
];

fn scaled_scene_rect(rect: (f32, f32, f32, f32), scale: f32) -> (f32, f32, f32, f32) {
    let width = rect.2 * scale;
    let height = rect.3 * scale;
    (
        rect.0 + (rect.2 - width) * 0.5,
        rect.1 + (rect.3 - height) * 0.5,
        width,
        height,
    )
}

impl DistrictLayout {
    fn render_rect(self) -> (f32, f32, f32, f32) {
        scaled_scene_rect(
            (self.asset_x, self.asset_y, self.asset_w, self.asset_h),
            LANDMARK_RENDER_SCALE,
        )
    }

    fn contains(self, position: Point<f32>) -> bool {
        let rect = self.render_rect();
        position.x >= rect.0
            && position.x <= rect.0 + rect.2
            && position.y >= rect.1
            && position.y <= rect.1 + rect.3
    }
}

#[derive(Clone)]
struct SceneMemoryLabel {
    category: MemoryBankCategory,
    title: String,
    status: MemoryBankStatus,
}

pub struct MemoryBankCanvasPanel {
    focus_handle: gpui::FocusHandle,
    start_time: Instant,
    selected_district: Option<usize>,
    items: Vec<MemoryBankItem>,
    links: Vec<MemoryBankLink>,
    category_counts: Vec<(MemoryBankCategory, usize)>,
    show_detail: bool,
    detail_content: String,
    detail_title: String,
    editing_item_id: Option<uuid::Uuid>,
    edit_title_input: Entity<InputState>,
    edit_content_input: Entity<InputState>,
    search_input: Entity<InputState>,
    is_editing: bool,
    selected_instance_id: String,
    view_mode: MemoryViewMode,
    category_filter: Option<MemoryBankCategory>,
    zoom: f32,
    scene_bounds: Arc<Mutex<Option<Bounds<Pixels>>>>,
    _subscriptions: Vec<Subscription>,
}

#[derive(Clone, Copy)]
struct MemoryWorldPalette {
    background: Hsla,
    grid: Hsla,
    border: Hsla,
    text: Hsla,
    text_dim: Hsla,
    core: Hsla,
}

impl MemoryWorldPalette {
    fn from_theme(theme: &gpui_component::Theme) -> Self {
        if theme.background.l < 0.5 {
            Self {
                background: Hsla::from(gpui::rgba(0x080d12ff)),
                grid: Hsla::from(gpui::rgba(0x5f74851c)),
                border: Hsla::from(gpui::rgba(0x7590a04a)),
                text: theme.foreground,
                text_dim: theme.muted_foreground,
                core: Hsla::from(gpui::rgba(0x37b8eaff)),
            }
        } else {
            Self {
                background: Hsla::from(gpui::rgba(0xeaf0f3ff)),
                grid: Hsla::from(gpui::rgba(0x46606f22)),
                border: Hsla::from(gpui::rgba(0x37505e55)),
                text: theme.foreground,
                text_dim: theme.muted_foreground,
                core: Hsla::from(gpui::rgba(0x0787b3ff)),
            }
        }
    }
}

#[derive(Clone, Copy)]
struct SceneTransform {
    ox: f32,
    oy: f32,
    scale: f32,
}

impl SceneTransform {
    fn new(bounds: Bounds<Pixels>, zoom: f32) -> Self {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        let base_scale = (width / SCENE_WIDTH).min(height / SCENE_HEIGHT);
        let scale = base_scale * zoom;
        let origin_x: f32 = bounds.origin.x.into();
        let origin_y: f32 = bounds.origin.y.into();
        Self {
            ox: origin_x + (width - SCENE_WIDTH * scale) / 2.0,
            oy: origin_y + (height - SCENE_HEIGHT * scale) / 2.0,
            scale,
        }
    }

    fn point(self, x: f32, y: f32) -> Point<Pixels> {
        point(px(self.ox + x * self.scale), px(self.oy + y * self.scale))
    }

    fn inverse(self, point: Point<Pixels>) -> Point<f32> {
        let x: f32 = point.x.into();
        let y: f32 = point.y.into();
        Point {
            x: (x - self.ox) / self.scale,
            y: (y - self.oy) / self.scale,
        }
    }

    fn local_rect(
        self,
        bounds: Bounds<Pixels>,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> (Pixels, Pixels, Pixels, Pixels) {
        let bounds_x: f32 = bounds.origin.x.into();
        let bounds_y: f32 = bounds.origin.y.into();
        (
            px(self.ox - bounds_x + x * self.scale),
            px(self.oy - bounds_y + y * self.scale),
            px(width * self.scale),
            px(height * self.scale),
        )
    }
}

impl MemoryBankCanvasPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let db = &crate::AppState::global(cx).db;
        let selected_instance_id = db
            .list_instances()
            .ok()
            .and_then(|list| list.first().map(|instance| instance.id.clone()))
            .unwrap_or_else(|| "default_instance".to_string());
        let edit_title_input = cx.new(|cx| InputState::new(window, cx).placeholder("Title..."));
        let edit_content_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(14)
                .soft_wrap(true)
                .placeholder("Write memory content...")
        });
        let search_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search memories...")
                .clean_on_escape()
        });

        let mut panel = Self {
            focus_handle: cx.focus_handle(),
            start_time: Instant::now(),
            selected_district: None,
            items: Vec::new(),
            links: Vec::new(),
            category_counts: Vec::new(),
            show_detail: false,
            detail_content: String::new(),
            detail_title: String::new(),
            editing_item_id: None,
            edit_title_input,
            edit_content_input,
            search_input,
            is_editing: false,
            selected_instance_id,
            view_mode: MemoryViewMode::Map,
            category_filter: None,
            zoom: 1.0,
            scene_bounds: Arc::new(Mutex::new(None)),
            _subscriptions: Vec::new(),
        };
        let search_subscription = cx.subscribe_in(
            &panel.search_input,
            window,
            |_this, _, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            },
        );
        panel._subscriptions.push(search_subscription);
        panel.load_data(cx);
        panel
    }

    fn load_data(&mut self, cx: &mut Context<Self>) {
        let service = &crate::AppState::global(cx).memory_bank_service;
        let _ = service.initialize_for_instance(&self.selected_instance_id);
        if let Ok(items) = service.list_items(&self.selected_instance_id) {
            self.items = items;
        }
        if let Ok(links) = service.list_links(&self.selected_instance_id) {
            self.links = links;
        }
        if let Ok(counts) = service.category_summary(&self.selected_instance_id) {
            self.category_counts = counts;
        }
        cx.notify();
    }

    fn selected_category(&self) -> Option<MemoryBankCategory> {
        self.selected_district
            .and_then(|index| DISTRICTS.get(index))
            .map(|layout| layout.category)
    }

    fn select_category(&mut self, category: MemoryBankCategory, cx: &mut Context<Self>) {
        self.selected_district = DISTRICTS
            .iter()
            .position(|layout| layout.category == category);
        self.show_detail = true;
        self.is_editing = false;
        if let Some(item) = self
            .items
            .iter()
            .find(|item| item.category == category && item.status == MemoryBankStatus::Active)
        {
            self.detail_title = item.title.clone();
            self.detail_content = item.content.clone();
            self.editing_item_id = Some(item.id);
        } else {
            self.detail_title = category_short_name(category).to_string();
            self.detail_content =
                "*No active memories in this district. Select Edit to create one.*".to_string();
            self.editing_item_id = None;
        }
        cx.notify();
    }

    fn select_item(&mut self, item_id: uuid::Uuid, cx: &mut Context<Self>) {
        if let Some(item) = self.items.iter().find(|item| item.id == item_id) {
            self.selected_district = DISTRICTS
                .iter()
                .position(|layout| layout.category == item.category);
            self.show_detail = true;
            self.is_editing = false;
            self.detail_title = item.title.clone();
            self.detail_content = item.content.clone();
            self.editing_item_id = Some(item.id);
            cx.notify();
        }
    }

    fn scene_point(&self, position: Point<Pixels>) -> Option<Point<f32>> {
        let bounds = self.scene_bounds.lock().ok()?.as_ref().copied()?;
        Some(SceneTransform::new(bounds, self.zoom).inverse(position))
    }

    fn handle_mouse_down(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(position) = self.scene_point(position) else {
            return;
        };
        let selected = DISTRICTS
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, layout)| {
                if self
                    .category_filter
                    .is_some_and(|category| category != layout.category)
                {
                    return None;
                }
                layout.contains(position).then_some(index)
            });
        if let Some(index) = selected {
            self.select_category(DISTRICTS[index].category, cx);
        } else {
            self.selected_district = None;
            self.show_detail = false;
            cx.notify();
        }
    }

    fn save_editing_item(&mut self, cx: &mut Context<Self>) {
        let title = self.edit_title_input.read(cx).text().to_string();
        let content = self.edit_content_input.read(cx).text().to_string();
        if title.trim().is_empty() || content.trim().is_empty() {
            return;
        }

        let service = &crate::AppState::global(cx).memory_bank_service;
        if let Some(item_id) = self.editing_item_id {
            if let Ok(Some(mut item)) = service.get_item(&item_id.to_string()) {
                item.title = title.clone();
                item.content = content.clone();
                item.updated_at = chrono::Utc::now();
                item.content_hash = MemoryBankItem::compute_hash(&item.content);
                item.token_count = MemoryBankItem::estimate_tokens(&item.content);
                let _ = service.update_item(&item);
            }
        } else if let Some(category) = self.selected_category() {
            let item = MemoryBankItem::new(
                &self.selected_instance_id,
                category,
                &title,
                &content,
                "user",
            );
            let _ = service.create_item(&item);
            self.editing_item_id = Some(item.id);
        }
        self.detail_title = title;
        self.detail_content = content;
        self.is_editing = false;
        self.load_data(cx);
    }

    fn enter_edit_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.is_editing = true;
        let title = self
            .editing_item_id
            .is_some()
            .then(|| self.detail_title.clone())
            .unwrap_or_default();
        let content = self
            .editing_item_id
            .is_some()
            .then(|| self.detail_content.clone())
            .unwrap_or_default();
        self.edit_title_input.update(cx, |input, cx| {
            input.set_value(&title, window, cx);
        });
        self.edit_content_input.update(cx, |input, cx| {
            input.set_value(&content, window, cx);
        });
        cx.notify();
    }

    fn filtered_items(&self, cx: &App) -> Vec<&MemoryBankItem> {
        let query = self
            .search_input
            .read(cx)
            .text()
            .to_string()
            .trim()
            .to_ascii_lowercase();
        self.items
            .iter()
            .filter(|item| item.status == MemoryBankStatus::Active)
            .filter(|item| {
                self.category_filter
                    .is_none_or(|category| item.category == category)
            })
            .filter(|item| {
                query.is_empty()
                    || item.title.to_ascii_lowercase().contains(&query)
                    || item.content.to_ascii_lowercase().contains(&query)
            })
            .collect()
    }

    fn scene_labels(&self, cx: &App) -> Vec<SceneMemoryLabel> {
        let mut per_category = std::collections::HashMap::<&'static str, usize>::new();
        self.filtered_items(cx)
            .into_iter()
            .filter_map(|item| {
                let count = per_category.entry(item.category.as_str()).or_default();
                if *count >= 3 {
                    return None;
                }
                *count += 1;
                Some(SceneMemoryLabel {
                    category: item.category,
                    title: item.title.clone(),
                    status: item.status,
                })
            })
            .collect()
    }

    fn toolbar(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let panel = cx.entity().clone();
        let selected_filter = self.category_filter;
        let filter_label = selected_filter
            .map(category_short_name)
            .unwrap_or("All categories");
        let filter_trigger = Button::new("memory-category-filter")
            .small()
            .label(filter_label)
            .dropdown_caret(true)
            .tooltip("Filter memory districts");
        let filter = Popover::new("memory-category-filter-popover")
            .anchor(Corner::BottomLeft)
            .trigger(filter_trigger)
            .content(move |_state, _window, cx| {
                let popover = cx.entity().clone();
                let mut rows = v_flex().w(px(220.)).gap(px(2.));
                let choices = std::iter::once(None)
                    .chain(MemoryBankCategory::all().iter().copied().map(Some));
                for (index, category) in choices.enumerate() {
                    let label = category
                        .map(category_short_name)
                        .unwrap_or("All categories");
                    let selected = category == selected_filter;
                    let panel = panel.clone();
                    let popover = popover.clone();
                    rows = rows.child(
                        h_flex()
                            .id(("memory-filter-option", index))
                            .w_full()
                            .h(px(30.))
                            .px(px(8.))
                            .gap(px(8.))
                            .items_center()
                            .rounded(px(5.))
                            .cursor_pointer()
                            .when(selected, |row| row.bg(cx.theme().primary.opacity(0.12)))
                            .hover(|row| row.bg(cx.theme().secondary))
                            .when_some(category, |row, category| {
                                row.child(
                                    div()
                                        .w(px(8.))
                                        .h(px(8.))
                                        .rounded_full()
                                        .bg(category_color(category)),
                                )
                            })
                            .child(div().text_size(px(12.)).child(label))
                            .on_click(move |_, window, cx| {
                                let _ = panel.update(cx, |this, cx| {
                                    this.category_filter = category;
                                    cx.notify();
                                });
                                let _ = popover.update(cx, |state, cx| {
                                    state.dismiss(window, cx);
                                });
                            }),
                    );
                }
                rows
            });

        h_flex()
            .w_full()
            .h(px(58.))
            .min_h(px(58.))
            .px(px(16.))
            .gap(px(12.))
            .items_center()
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.background)
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(px(17.))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("Memory Bank"),
            )
            .child(
                div().min_w(px(180.)).max_w(px(360.)).flex_1().child(
                    Input::new(&self.search_input)
                        .small()
                        .prefix(IconName::Search),
                ),
            )
            .child(filter)
            .child(
                h_flex()
                    .flex_shrink_0()
                    .p(px(2.))
                    .gap(px(2.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(theme.border)
                    .child(
                        Button::new("memory-view-map")
                            .small()
                            .compact()
                            .selected(self.view_mode == MemoryViewMode::Map)
                            .label("Map")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.view_mode = MemoryViewMode::Map;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("memory-view-graph")
                            .small()
                            .compact()
                            .selected(self.view_mode == MemoryViewMode::Graph)
                            .label("Graph")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.view_mode = MemoryViewMode::Graph;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("memory-view-list")
                            .small()
                            .compact()
                            .selected(self.view_mode == MemoryViewMode::List)
                            .label("List")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.view_mode = MemoryViewMode::List;
                                cx.notify();
                            })),
                    ),
            )
            .into_any_element()
    }

    fn canvas_view(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let view = cx.entity().clone();
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let selected = self.selected_district;
        let counts = self.category_counts.clone();
        let labels = self.scene_labels(cx);
        let category_filter = self.category_filter;
        let zoom = self.zoom;
        let graph_mode = self.view_mode == MemoryViewMode::Graph;
        let link_count = self.links.len();
        let scene_bounds = self.scene_bounds.clone();
        let asset_layer = self.asset_layer();
        let foreground_counts = counts.clone();
        let foreground_labels = labels.clone();

        div()
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_hidden()
            .bg(theme.background)
            .on_mouse_down(MouseButton::Left, {
                let view = view.clone();
                move |event, _, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.handle_mouse_down(event.position, cx);
                    });
                }
            })
            .child(
                canvas(
                    |_bounds, _window, _cx| {},
                    move |bounds, _, window, cx| {
                        if let Ok(mut stored_bounds) = scene_bounds.lock() {
                            *stored_bounds = Some(bounds);
                        }
                        window.with_content_mask(Some(ContentMask { bounds }), |window| {
                            paint_memory_world_background(bounds, zoom, window, cx);
                        });
                    },
                )
                .size_full(),
            )
            .child(asset_layer)
            .child(
                canvas(
                    |_bounds, _window, _cx| {},
                    move |bounds, _, window, cx| {
                        window.with_content_mask(Some(ContentMask { bounds }), |window| {
                            paint_memory_world_foreground(
                                bounds,
                                elapsed,
                                selected,
                                &foreground_counts,
                                &foreground_labels,
                                category_filter,
                                zoom,
                                graph_mode,
                                link_count,
                                window,
                                cx,
                            );
                        });
                    },
                )
                .absolute()
                .top(px(0.))
                .left(px(0.))
                .size_full(),
            )
            .child(
                v_flex()
                    .absolute()
                    .bottom(px(18.))
                    .left(px(18.))
                    .gap(px(4.))
                    .p(px(4.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background.opacity(0.94))
                    .child(
                        Button::new("memory-zoom-in")
                            .xsmall()
                            .compact()
                            .label("+")
                            .tooltip("Zoom in")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.zoom = (this.zoom + 0.1).min(1.5);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("memory-zoom-out")
                            .xsmall()
                            .compact()
                            .label("-")
                            .tooltip("Zoom out")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.zoom = (this.zoom - 0.1).max(0.7);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("memory-zoom-fit")
                            .xsmall()
                            .compact()
                            .icon(IconName::Replace)
                            .tooltip("Fit scene")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.zoom = 1.0;
                                cx.notify();
                            })),
                    ),
            )
            .into_any_element()
    }

    fn asset_layer(&self) -> gpui::AnyElement {
        let bounds = self
            .scene_bounds
            .lock()
            .ok()
            .and_then(|stored_bounds| *stored_bounds);
        let Some(bounds) = bounds else {
            return div()
                .absolute()
                .top(px(0.))
                .left(px(0.))
                .size_full()
                .into_any_element();
        };

        let transform = SceneTransform::new(bounds, self.zoom);
        let mut layer = div()
            .absolute()
            .top(px(0.))
            .left(px(0.))
            .size_full()
            .overflow_hidden();

        for bridge in BRIDGE_ASSETS {
            let dimmed = self
                .category_filter
                .is_some_and(|category| category != bridge.category);
            layer = layer.child(positioned_memory_asset(
                bridge.asset_path,
                transform.local_rect(bounds, bridge.x, bridge.y, bridge.width, bridge.height),
                if dimmed { 0.10 } else { 0.94 },
            ));
        }

        for index in [1usize, 0, 2] {
            let district = DISTRICTS[index];
            let dimmed = self
                .category_filter
                .is_some_and(|category| category != district.category);
            let rect = district.render_rect();
            layer = layer.child(positioned_memory_asset(
                district.asset_path,
                transform.local_rect(bounds, rect.0, rect.1, rect.2, rect.3),
                if dimmed { 0.16 } else { 1.0 },
            ));
        }

        let core_rect = scaled_scene_rect(CORE_ASSET_RECT, LANDMARK_RENDER_SCALE);
        layer = layer.child(positioned_memory_asset(
            CORE_ASSET_PATH,
            transform.local_rect(bounds, core_rect.0, core_rect.1, core_rect.2, core_rect.3),
            if self.category_filter.is_some() {
                0.78
            } else {
                1.0
            },
        ));

        for index in [3usize, 5, 4] {
            let district = DISTRICTS[index];
            let dimmed = self
                .category_filter
                .is_some_and(|category| category != district.category);
            let rect = district.render_rect();
            layer = layer.child(positioned_memory_asset(
                district.asset_path,
                transform.local_rect(bounds, rect.0, rect.1, rect.2, rect.3),
                if dimmed { 0.16 } else { 1.0 },
            ));
        }

        for rect in SENTINEL_RECTS {
            layer = layer.child(positioned_memory_asset(
                SENTINEL_ASSET_PATH,
                transform.local_rect(bounds, rect.0, rect.1, rect.2, rect.3),
                1.0,
            ));
        }

        layer.into_any_element()
    }

    fn list_view(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let items = self.filtered_items(cx);
        let mut list = v_flex().w_full().gap(px(1.));
        if items.is_empty() {
            list = list.child(
                v_flex()
                    .w_full()
                    .items_center()
                    .justify_center()
                    .gap(px(8.))
                    .py(px(80.))
                    .child(Icon::new(IconName::BookOpen).size(px(28.)))
                    .child(div().text_size(px(12.)).child("No matching memories")),
            );
        }
        for item in items {
            let item_id = item.id;
            let color = category_color(item.category);
            let selected = self.editing_item_id == Some(item.id);
            list = list.child(
                h_flex()
                    .id(gpui::ElementId::Name(
                        format!("memory-list-{}", item.id).into(),
                    ))
                    .w_full()
                    .min_w_0()
                    .h(px(58.))
                    .px(px(14.))
                    .gap(px(10.))
                    .items_center()
                    .border_b_1()
                    .border_color(theme.border)
                    .cursor_pointer()
                    .when(selected, |row| row.bg(theme.primary.opacity(0.08)))
                    .hover(|row| row.bg(theme.secondary.opacity(0.6)))
                    .child(div().w(px(4.)).h(px(30.)).rounded(px(2.)).bg(color))
                    .child(
                        v_flex()
                            .min_w_0()
                            .flex_1()
                            .gap(px(3.))
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(12.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(item.title.clone()),
                            )
                            .child(
                                h_flex()
                                    .gap(px(8.))
                                    .text_size(px(9.))
                                    .text_color(theme.muted_foreground)
                                    .child(category_short_name(item.category))
                                    .child(format!("v{}", item.version))
                                    .child(format!("{} tokens", item.token_count)),
                            ),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(9.))
                            .text_color(theme.muted_foreground)
                            .child(item.updated_at.format("%b %d, %H:%M").to_string()),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select_item(item_id, cx);
                    })),
            );
        }

        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scrollbar()
            .child(list)
            .into_any_element()
    }

    fn inspector(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let category = self.selected_category();
        let selected_item = self
            .editing_item_id
            .and_then(|item_id| self.items.iter().find(|item| item.id == item_id));
        let category_items = category
            .map(|category| {
                self.items
                    .iter()
                    .filter(|item| {
                        item.category == category && item.status == MemoryBankStatus::Active
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let linked_items = selected_item
            .map(|selected| {
                self.links
                    .iter()
                    .filter_map(|link| {
                        let other_id = if link.source_id == selected.id {
                            Some(link.target_id)
                        } else if link.target_id == selected.id {
                            Some(link.source_id)
                        } else {
                            None
                        }?;
                        self.items.iter().find(|item| item.id == other_id)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let has_linked_items = !linked_items.is_empty();

        let mut category_list = v_flex().w_full().gap(px(2.));
        for item in category_items {
            let item_id = item.id;
            let selected = self.editing_item_id == Some(item.id);
            category_list = category_list.child(
                h_flex()
                    .id(gpui::ElementId::Name(
                        format!("memory-inspector-{}", item.id).into(),
                    ))
                    .w_full()
                    .min_w_0()
                    .h(px(34.))
                    .px(px(8.))
                    .gap(px(8.))
                    .items_center()
                    .rounded(px(5.))
                    .cursor_pointer()
                    .when(selected, |row| row.bg(theme.primary.opacity(0.10)))
                    .hover(|row| row.bg(theme.secondary))
                    .child(Icon::new(IconName::File).size(px(13.)))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .truncate()
                            .text_size(px(11.))
                            .child(item.title.clone()),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select_item(item_id, cx);
                    })),
            );
        }

        let mut linked_list = v_flex().w_full().gap(px(2.));
        for item in linked_items {
            let item_id = item.id;
            linked_list = linked_list.child(
                h_flex()
                    .id(gpui::ElementId::Name(
                        format!("memory-linked-{}", item.id).into(),
                    ))
                    .w_full()
                    .min_w_0()
                    .h(px(32.))
                    .px(px(8.))
                    .gap(px(8.))
                    .items_center()
                    .rounded(px(5.))
                    .cursor_pointer()
                    .hover(|row| row.bg(theme.secondary))
                    .child(
                        div()
                            .w(px(7.))
                            .h(px(7.))
                            .rounded_full()
                            .bg(category_color(item.category)),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .truncate()
                            .text_size(px(10.))
                            .child(item.title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(9.))
                            .text_color(theme.muted_foreground)
                            .child(category_short_name(item.category)),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select_item(item_id, cx);
                    })),
            );
        }

        v_flex()
            .w(px(390.))
            .min_w(px(320.))
            .h_full()
            .border_l_1()
            .border_color(theme.border)
            .bg(theme.background)
            .child(
                h_flex()
                    .w_full()
                    .h(px(58.))
                    .px(px(14.))
                    .gap(px(8.))
                    .items_center()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .truncate()
                            .text_size(px(14.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(self.detail_title.clone()),
                    )
                    .child(
                        Button::new("memory-inspector-edit")
                            .small()
                            .compact()
                            .ghost()
                            .icon(IconName::Settings2)
                            .tooltip("Edit memory")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.enter_edit_mode(window, cx);
                            })),
                    )
                    .child(
                        Button::new("memory-inspector-close")
                            .small()
                            .compact()
                            .ghost()
                            .icon(IconName::Close)
                            .tooltip("Close inspector")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_detail = false;
                                this.selected_district = None;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div().flex_1().min_h_0().overflow_y_scrollbar().child(
                    v_flex()
                        .w_full()
                        .gap(px(14.))
                        .p(px(14.))
                        .when_some(category, |column, category| {
                            column.child(
                                h_flex()
                                    .gap(px(7.))
                                    .items_center()
                                    .child(
                                        div()
                                            .w(px(9.))
                                            .h(px(9.))
                                            .rounded(px(2.))
                                            .bg(category_color(category)),
                                    )
                                    .child(
                                        div()
                                            .px(px(7.))
                                            .py(px(3.))
                                            .rounded(px(4.))
                                            .bg(category_color(category).opacity(0.12))
                                            .text_size(px(10.))
                                            .text_color(category_color(category))
                                            .child(category_short_name(category)),
                                    ),
                            )
                        })
                        .when_some(selected_item, |column, item| {
                            column.child(
                                h_flex()
                                    .w_full()
                                    .justify_between()
                                    .text_size(px(9.))
                                    .text_color(theme.muted_foreground)
                                    .child(format!("Version {}", item.version))
                                    .child(
                                        item.updated_at
                                            .format("Updated %b %d, %Y %H:%M")
                                            .to_string(),
                                    ),
                            )
                        })
                        .child(if self.is_editing {
                            v_flex()
                                .w_full()
                                .gap(px(10.))
                                .child(Input::new(&self.edit_title_input))
                                .child(Input::new(&self.edit_content_input).h(px(300.)))
                                .child(
                                    h_flex()
                                        .gap(px(8.))
                                        .justify_end()
                                        .child(
                                            Button::new("memory-edit-cancel")
                                                .small()
                                                .label("Cancel")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.is_editing = false;
                                                    cx.notify();
                                                })),
                                        )
                                        .child(
                                            Button::new("memory-edit-save")
                                                .small()
                                                .primary()
                                                .label("Save")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.save_editing_item(cx);
                                                })),
                                        ),
                                )
                                .into_any_element()
                        } else {
                            div()
                                .w_full()
                                .text_size(px(11.))
                                .line_height(gpui::relative(1.45))
                                .child(
                                    TextView::markdown(
                                        gpui::ElementId::Name("memory-detail-preview".into()),
                                        self.detail_content.clone(),
                                    )
                                    .w_full()
                                    .selectable(true),
                                )
                                .into_any_element()
                        })
                        .child(
                            v_flex()
                                .w_full()
                                .gap(px(7.))
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(theme.muted_foreground)
                                        .child("Memories in district"),
                                )
                                .child(category_list),
                        )
                        .when(has_linked_items, |column| {
                            column.child(
                                v_flex()
                                    .w_full()
                                    .gap(px(7.))
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(theme.muted_foreground)
                                            .child("Linked memories"),
                                    )
                                    .child(linked_list),
                            )
                        }),
                ),
            )
            .into_any_element()
    }
}

impl Panel for MemoryBankCanvasPanel {
    fn panel_name(&self) -> &'static str {
        "Memory Bank"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.panel_name()
    }

    fn title_style(&self, _cx: &App) -> Option<TitleStyle> {
        None
    }
}

impl Focusable for MemoryBankCanvasPanel {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for MemoryBankCanvasPanel {}

impl Render for MemoryBankCanvasPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.view_mode != MemoryViewMode::List {
            let view = cx.entity().clone();
            cx.spawn(async move |_, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(66))
                    .await;
                let _ = cx.update(|cx| {
                    let _ = view.update(cx, |_, cx| cx.notify());
                });
            })
            .detach();
        }
        let theme = cx.theme().clone();
        let main_content = if self.view_mode == MemoryViewMode::List {
            self.list_view(cx)
        } else {
            self.canvas_view(cx)
        };

        v_flex()
            .size_full()
            .min_w_0()
            .bg(theme.background)
            .child(self.toolbar(cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .child(main_content)
                    .when(self.show_detail, |row| row.child(self.inspector(cx))),
            )
    }
}

fn category_short_name(category: MemoryBankCategory) -> &'static str {
    match category {
        MemoryBankCategory::ProjectBrief => "Project Brief",
        MemoryBankCategory::SystemArchitecture => "Architecture",
        MemoryBankCategory::ActiveContext => "Context",
        MemoryBankCategory::Progress => "Progress",
        MemoryBankCategory::LessonsLearned => "Lessons",
        MemoryBankCategory::DecisionLog => "Decisions",
    }
}

fn category_color(category: MemoryBankCategory) -> Hsla {
    match category {
        MemoryBankCategory::ProjectBrief => Hsla::from(gpui::rgb(0x22d3ee)),
        MemoryBankCategory::SystemArchitecture => Hsla::from(gpui::rgb(0x7ddc66)),
        MemoryBankCategory::ActiveContext => Hsla::from(gpui::rgb(0xf3b53f)),
        MemoryBankCategory::Progress => Hsla::from(gpui::rgb(0xff754f)),
        MemoryBankCategory::LessonsLearned => Hsla::from(gpui::rgb(0xec4899)),
        MemoryBankCategory::DecisionLog => Hsla::from(gpui::rgb(0x38bdf8)),
    }
}

fn positioned_memory_asset(
    path: &'static str,
    rect: (Pixels, Pixels, Pixels, Pixels),
    opacity: f32,
) -> gpui::AnyElement {
    img(path)
        .absolute()
        .left(rect.0)
        .top(rect.1)
        .w(rect.2)
        .h(rect.3)
        .object_fit(ObjectFit::Contain)
        .opacity(opacity)
        .into_any_element()
}

fn paint_rect(
    window: &mut Window,
    x: Pixels,
    y: Pixels,
    width: Pixels,
    height: Pixels,
    color: Hsla,
) {
    window.paint_quad(fill(
        Bounds::from_corners(point(x, y), point(x + width, y + height)),
        color,
    ));
}

fn paint_rounded_rect(
    window: &mut Window,
    x: Pixels,
    y: Pixels,
    width: Pixels,
    height: Pixels,
    radius: Pixels,
    fill_color: Hsla,
    border_color: Hsla,
) {
    let bounds = Bounds::from_corners(point(x, y), point(x + width, y + height));
    window.paint_quad(quad(
        bounds,
        radius,
        fill_color,
        Edges::all(px(1.0)),
        border_color,
        BorderStyle::default(),
    ));
}

fn paint_line(
    window: &mut Window,
    from: Point<Pixels>,
    to: Point<Pixels>,
    width: f32,
    color: Hsla,
) {
    let mut builder = gpui::PathBuilder::stroke(px(width))
        .with_style(gpui::PathStyle::Stroke(gpui::StrokeOptions::default()));
    builder.move_to(from);
    builder.line_to(to);
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}

fn paint_circle(window: &mut Window, center: Point<Pixels>, radius: f32, color: Hsla) {
    let diameter = px(radius * 2.0);
    let bounds = Bounds::from_corners(
        point(center.x - px(radius), center.y - px(radius)),
        point(
            center.x - px(radius) + diameter,
            center.y - px(radius) + diameter,
        ),
    );
    window.paint_quad(quad(
        bounds,
        px(radius),
        color,
        Edges::all(px(0.0)),
        color,
        BorderStyle::default(),
    ));
}

fn paint_memory_world_background(
    bounds: Bounds<Pixels>,
    zoom: f32,
    window: &mut Window,
    cx: &mut App,
) {
    let palette = MemoryWorldPalette::from_theme(cx.theme());
    paint_rect(
        window,
        bounds.origin.x,
        bounds.origin.y,
        bounds.size.width,
        bounds.size.height,
        palette.background,
    );
    let transform = SceneTransform::new(bounds, zoom);

    for x in (-200..=1300).step_by(60) {
        paint_line(
            window,
            transform.point(x as f32, -120.0),
            transform.point(x as f32 + 520.0, 880.0),
            0.7,
            palette.grid,
        );
    }
    for x in (-200..=1300).step_by(60) {
        paint_line(
            window,
            transform.point(x as f32, -120.0),
            transform.point(x as f32 - 520.0, 880.0),
            0.7,
            palette.grid,
        );
    }
}

fn paint_memory_world_foreground(
    bounds: Bounds<Pixels>,
    elapsed: f32,
    selected: Option<usize>,
    category_counts: &[(MemoryBankCategory, usize)],
    labels: &[SceneMemoryLabel],
    category_filter: Option<MemoryBankCategory>,
    zoom: f32,
    graph_mode: bool,
    link_count: usize,
    window: &mut Window,
    cx: &mut App,
) {
    let palette = MemoryWorldPalette::from_theme(cx.theme());
    let transform = SceneTransform::new(bounds, zoom);

    for district in DISTRICTS.iter() {
        let dimmed = category_filter.is_some_and(|category| category != district.category);
        let color = category_color(district.category);
        let alpha = if dimmed { 0.24 } else { 1.0 };

        let count = category_counts
            .iter()
            .find(|(category, _)| *category == district.category)
            .map(|(_, count)| *count)
            .unwrap_or(0);
        paint_category_label(
            window,
            cx,
            transform,
            *district,
            count,
            color.opacity(alpha),
            palette,
        );
    }

    let core = (550.0, 385.0);
    let core_pulse = 10.0 + (elapsed * 2.4).sin().abs() * 5.0;
    paint_circle(
        window,
        transform.point(core.0, core.1 - 42.0),
        core_pulse,
        palette.core.opacity(0.22),
    );
    paint_circle(
        window,
        transform.point(core.0, core.1 - 42.0),
        4.0,
        palette.core,
    );
    paint_text_centered(
        window,
        cx,
        "Project Core",
        transform.point(core.0, core.1 + 82.0).x,
        transform.point(core.0, core.1 + 82.0).y,
        px(13.0),
        palette.text,
    );

    paint_memory_labels(window, cx, transform, labels, palette);
    paint_minimap(bounds, selected, category_filter, window, cx, palette);
    if graph_mode {
        paint_text_left(
            window,
            cx,
            &format!("{} semantic links", link_count),
            bounds.origin.x + px(86.0),
            bounds.origin.y + px(24.0),
            px(10.0),
            palette.text_dim,
        );
    }
}

fn paint_category_label(
    window: &mut Window,
    cx: &mut App,
    transform: SceneTransform,
    district: DistrictLayout,
    count: usize,
    color: Hsla,
    palette: MemoryWorldPalette,
) {
    let label = format!("{}  {}", category_short_name(district.category), count);
    let rect = district.render_rect();
    let center = transform.point(rect.0 + rect.2 * 0.5, rect.1 + rect.3 - 18.0);
    let width = (label.chars().count() as f32 * 7.0 + 24.0).max(82.0);
    paint_rounded_rect(
        window,
        center.x - px(width / 2.0),
        center.y - px(11.0),
        px(width),
        px(22.0),
        px(5.0),
        palette.background.opacity(0.92),
        color,
    );
    paint_text_centered(window, cx, &label, center.x, center.y, px(10.5), color);
}

fn paint_memory_labels(
    window: &mut Window,
    cx: &mut App,
    transform: SceneTransform,
    labels: &[SceneMemoryLabel],
    palette: MemoryWorldPalette,
) {
    let mut category_offsets = std::collections::HashMap::<&'static str, usize>::new();
    for label in labels {
        let index = *category_offsets.entry(label.category.as_str()).or_default();
        *category_offsets.entry(label.category.as_str()).or_default() += 1;
        let (x, y) = match label.category {
            MemoryBankCategory::ProjectBrief => (28.0, 118.0 + index as f32 * 29.0),
            MemoryBankCategory::SystemArchitecture => (688.0, 60.0 + index as f32 * 29.0),
            MemoryBankCategory::ActiveContext => (925.0, 145.0 + index as f32 * 29.0),
            MemoryBankCategory::Progress => (20.0, 420.0 + index as f32 * 29.0),
            MemoryBankCategory::LessonsLearned => (295.0, 664.0 + index as f32 * 29.0),
            MemoryBankCategory::DecisionLog => (925.0, 468.0 + index as f32 * 29.0),
        };
        let color = category_color(label.category);
        let box_origin = transform.point(x, y);
        let box_width = 152.0 * transform.scale;
        let box_height = 24.0 * transform.scale;
        let box_center = transform.point(x + 76.0, y + 12.0);
        paint_rounded_rect(
            window,
            box_origin.x,
            box_origin.y,
            px(box_width),
            px(box_height),
            px(4.0),
            palette.background.opacity(0.90),
            palette.border,
        );
        paint_circle(
            window,
            point(box_origin.x + px(10.0), box_center.y),
            2.5,
            if label.status == MemoryBankStatus::Active {
                color
            } else {
                palette.text_dim
            },
        );
        paint_text_left(
            window,
            cx,
            &truncate_label(&label.title, 23),
            box_origin.x + px(18.0),
            box_center.y,
            px(9.0),
            palette.text,
        );
    }
}

fn paint_minimap(
    bounds: Bounds<Pixels>,
    selected: Option<usize>,
    category_filter: Option<MemoryBankCategory>,
    window: &mut Window,
    cx: &mut App,
    palette: MemoryWorldPalette,
) {
    let x = bounds.origin.x + px(72.0);
    let y = bounds.origin.y + bounds.size.height - px(132.0);
    paint_rounded_rect(
        window,
        x,
        y,
        px(150.0),
        px(112.0),
        px(6.0),
        palette.background.opacity(0.92),
        palette.border,
    );
    let center = point(x + px(75.0), y + px(57.0));
    for (index, district) in DISTRICTS.iter().enumerate() {
        let node = point(
            x + px(12.0 + district.cx / SCENE_WIDTH * 126.0),
            y + px(10.0 + district.cy / SCENE_HEIGHT * 88.0),
        );
        let dimmed = category_filter.is_some_and(|category| category != district.category);
        paint_line(window, center, node, 0.8, palette.border);
        paint_circle(
            window,
            node,
            if selected == Some(index) { 4.0 } else { 3.0 },
            category_color(district.category).opacity(if dimmed { 0.2 } else { 0.9 }),
        );
    }
    paint_circle(window, center, 4.0, palette.core);
    paint_text_left(
        window,
        cx,
        "WORLD MAP",
        x + px(9.0),
        y + px(100.0),
        px(8.0),
        palette.text_dim,
    );
}

fn truncate_label(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let compact = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{}...", compact)
    } else {
        compact
    }
}

fn paint_text_centered(
    window: &mut Window,
    cx: &mut App,
    text: &str,
    center_x: Pixels,
    center_y: Pixels,
    font_size: Pixels,
    color: Hsla,
) {
    let text_style = gpui::TextStyle {
        font_size: font_size.into(),
        color,
        ..window.text_style().clone()
    };
    let run = gpui::TextRun {
        len: text.len(),
        font: text_style.font(),
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let shaped = window.text_system().shape_line(
        SharedString::from(text.to_string()),
        font_size,
        &[run],
        None,
    );
    let origin = point(center_x - shaped.width / 2.0, center_y - font_size / 2.0);
    let _ = shaped.paint(origin, font_size, window, cx);
}

fn paint_text_left(
    window: &mut Window,
    cx: &mut App,
    text: &str,
    x: Pixels,
    center_y: Pixels,
    font_size: Pixels,
    color: Hsla,
) {
    let text_style = gpui::TextStyle {
        font_size: font_size.into(),
        color,
        ..window.text_style().clone()
    };
    let run = gpui::TextRun {
        len: text.len(),
        font: text_style.font(),
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let shaped = window.text_system().shape_line(
        SharedString::from(text.to_string()),
        font_size,
        &[run],
        None,
    );
    let _ = shaped.paint(point(x, center_y - font_size / 2.0), font_size, window, cx);
}
