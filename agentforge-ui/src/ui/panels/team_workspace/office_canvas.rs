use gpui::prelude::FluentBuilder;
use gpui::{
    canvas, div, fill, point, px, quad, App, BorderStyle, Bounds, ContentMask, Context, Edges,
    Hsla, InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Point, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::Input;
use gpui_component::scroll::ScrollableElement;
use gpui_component::{h_flex, v_flex, ActiveTheme as _, IconName, Sizable};
use std::time::Instant;

// Constants

const MAP_COLS: usize = 32;
const MAP_ROWS: usize = 16;
const OFFICE_CANVAS_COVER: bool = true;
const STREAM_DISPLAY_MARKERS: [&str; 6] = [
    "<|channel>thought<channel|>",
    "<|channel>final<channel|>",
    "<|channel>analysis<channel|>",
    "<|start|>",
    "<|end|>",
    "<|message|>",
];

// Office Agent (visual representation)

#[derive(Clone, Debug)]
pub struct OfficeAgent {
    pub id: String,
    pub name: String,
    pub status: String,
    pub task_count: usize,
    ready_for_assignment: bool,
    pub color: Hsla,
    pub x: f32,
    pub y: f32,
    pub tx: f32,
    pub ty: f32,
    pub message: Option<String>,
    pub message_expires: Option<Instant>,
}

// Office State

#[derive(Clone, Debug)]
pub struct OfficeState {
    pub agents: Vec<OfficeAgent>,
    pub dragged_agent_idx: Option<usize>,
    start_time: Instant,
}

const COORDINATOR_SPOT: (f32, f32) = (17.2, 5.3);

const WAITING_SPOTS: [(f32, f32); 7] = [
    (14.2, 11.2),
    (16.0, 11.2),
    (17.8, 11.2),
    (19.6, 11.2),
    (15.1, 12.7),
    (17.0, 12.7),
    (18.9, 12.7),
];

const WORK_SPOTS: [(f32, f32); 7] = [
    (3.5, 7.5),
    (8.4, 7.5),
    (3.5, 10.2),
    (8.4, 10.2),
    (26.0, 8.0),
    (27.6, 8.0),
    (17.8, 6.7),
];

const AGENT_COLORS: [u32; 7] = [
    0xf472b6ff, 0xa78bfaff, 0x60a5faff, 0x4ade80ff, 0xfbbf24ff, 0xfb923cff, 0x34d399ff,
];

fn is_coordinator_identity(id: &str, name: &str) -> bool {
    let id = id.to_ascii_lowercase();
    let name = name.to_ascii_lowercase();
    id.contains("coord") || name.contains("coord")
}

fn is_waiting_status(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "" | "online" | "idle" | "waiting" | "available" | "standby" | "offline"
    )
}

fn waiting_spot(index: usize) -> (f32, f32) {
    WAITING_SPOTS[index % WAITING_SPOTS.len()]
}

fn work_spot(index: usize, name: &str, status: &str) -> (f32, f32) {
    let key = format!("{} {}", name, status).to_ascii_lowercase();
    if key.contains("pm") || key.contains("manager") || key.contains("review") {
        return WORK_SPOTS[4 + index % 2];
    }
    if key.contains("communicat") || key.contains("plan") {
        return WORK_SPOTS[6];
    }
    WORK_SPOTS[index % 4]
}

fn target_spot(
    index: usize,
    id: &str,
    name: &str,
    status: &str,
    task_count: usize,
    ready_for_assignment: bool,
) -> (f32, f32) {
    if is_coordinator_identity(id, name) {
        return COORDINATOR_SPOT;
    }
    if !ready_for_assignment || (task_count == 0 && is_waiting_status(status)) {
        return waiting_spot(index);
    }
    work_spot(index, name, status)
}

impl OfficeState {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            dragged_agent_idx: None,
            start_time: Instant::now(),
        }
    }

    pub fn elapsed_secs(&self) -> f32 {
        self.start_time.elapsed().as_secs_f32()
    }

    /// Sync agent list from the database agent records.
    pub fn sync_agents(&mut self, agent_data: Vec<(String, String, String, usize)>) {
        // agent_data: Vec<(id, name, status, active_task_count)>
        let mut new_ids: Vec<String> = Vec::new();
        for (i, (id, name, status, task_count)) in agent_data.iter().enumerate() {
            new_ids.push(id.clone());
            if let Some(existing) = self.agents.iter_mut().find(|a| a.id == *id) {
                existing.status = status.clone();
                existing.task_count = *task_count;
                if !name.is_empty() {
                    existing.name = name.clone();
                }
                let target = target_spot(
                    i,
                    &existing.id,
                    &existing.name,
                    &existing.status,
                    existing.task_count,
                    existing.ready_for_assignment,
                );
                existing.tx = target.0;
                existing.ty = target.1;
                if is_coordinator_identity(&existing.id, &existing.name) {
                    existing.x = target.0;
                    existing.y = target.1;
                }
            } else {
                let color = Hsla::from(gpui::rgba(AGENT_COLORS[i % AGENT_COLORS.len()]));
                let is_coordinator = is_coordinator_identity(id, name);
                let spot = if is_coordinator {
                    COORDINATOR_SPOT
                } else {
                    waiting_spot(i)
                };
                self.agents.push(OfficeAgent {
                    id: id.clone(),
                    name: name.clone(),
                    status: status.clone(),
                    task_count: *task_count,
                    ready_for_assignment: is_coordinator,
                    color,
                    x: spot.0,
                    y: spot.1,
                    tx: spot.0,
                    ty: spot.1,
                    message: None,
                    message_expires: None,
                });
            }
        }
        self.agents.retain(|a| new_ids.contains(&a.id));
        if self
            .dragged_agent_idx
            .is_some_and(|idx| idx >= self.agents.len())
        {
            self.dragged_agent_idx = None;
        }
    }

    /// Push a chat message bubble onto an agent.
    pub fn push_message(&mut self, agent_id: &str, text: &str) {
        let Some(preview) = office_visible_message_text(text, 34) else {
            return;
        };
        let target_idx = self
            .agents
            .iter()
            .position(|a| a.id == agent_id)
            .or_else(|| (!self.agents.is_empty()).then_some(0));
        if let Some(idx) = target_idx {
            let agent = &mut self.agents[idx];
            agent.message = Some(preview);
            agent.message_expires = Some(Instant::now() + std::time::Duration::from_secs(6));
        }
    }

    pub fn reset(&mut self) {
        self.agents.clear();
        self.dragged_agent_idx = None;
    }

    /// Update interpolation step (call each frame).
    fn tick(&mut self) {
        // Expire messages
        let now = Instant::now();
        for agent in &mut self.agents {
            if let Some(expires) = agent.message_expires {
                if now >= expires {
                    agent.message = None;
                    agent.message_expires = None;
                }
            }
        }
        // Smooth movement
        for agent in &mut self.agents {
            agent.ready_for_assignment = true;
            let dx = agent.tx - agent.x;
            let dy = agent.ty - agent.y;
            agent.x = if dx.abs() <= 0.01 {
                agent.tx
            } else {
                agent.x + dx * 0.1
            };
            agent.y = if dy.abs() <= 0.01 {
                agent.ty
            } else {
                agent.y + dy * 0.1
            };
        }
    }

    fn has_active_motion(&self) -> bool {
        self.dragged_agent_idx.is_some()
            || self
                .agents
                .iter()
                .any(|agent| (agent.x - agent.tx).abs() > 0.01 || (agent.y - agent.ty).abs() > 0.01)
    }

    fn has_live_messages(&self) -> bool {
        self.agents
            .iter()
            .any(|agent| agent.message_expires.is_some())
    }

    fn needs_animation_tick(&self) -> bool {
        self.has_active_motion() || self.has_live_messages()
    }
}

// Palette (derived from theme)

struct OfficePalette {
    bg: Hsla,
    wall_front: Hsla,
    wall_shadow: Hsla,
    wall_highlight: Hsla,
    floor_dev: Hsla,
    floor_lounge: Hsla,
    floor_manager: Hsla,
    floor_kitchen: Hsla,
    floor_grid: Hsla,
    floor_checker_light: Hsla,
    floor_checker_dark: Hsla,
    desk_top: Hsla,
    desk_base: Hsla,
    desk_edge: Hsla,
    desk_light: Hsla,
    pc_bezel: Hsla,
    pc_screen: Hsla,
    pc_glow: Hsla,
    plant_pot: Hsla,
    leaf1: Hsla,
    leaf2: Hsla,
    leaf3: Hsla,
    sofa_base: Hsla,
    sofa_top: Hsla,
    sofa_hl: Hsla,
    shelf_base: Hsla,
    shelf_wood: Hsla,
    text: Hsla,
    text_dim: Hsla,
    accent: Hsla,
    green: Hsla,
    label_bg: Hsla,
}

impl OfficePalette {
    fn from_theme(theme: &gpui_component::Theme) -> Self {
        let is_dark = theme.background.l < 0.5;
        if is_dark {
            Self {
                bg: Hsla::from(gpui::rgba(0x05080fff)),
                wall_front: Hsla::from(gpui::rgba(0x142334ff)),
                wall_shadow: Hsla::from(gpui::rgba(0x050b12ff)),
                wall_highlight: Hsla::from(gpui::rgba(0x2f4a5fff)),
                floor_dev: Hsla::from(gpui::rgba(0x3d6385ff)),
                floor_lounge: Hsla::from(gpui::rgba(0x78451fff)),
                floor_manager: Hsla::from(gpui::rgba(0x3a607fff)),
                floor_kitchen: Hsla::from(gpui::rgba(0x252d37ff)),
                floor_grid: Hsla::from(gpui::rgba(0x17324aff)),
                floor_checker_light: Hsla::from(gpui::rgba(0xd5dce2ff)),
                floor_checker_dark: Hsla::from(gpui::rgba(0x171d25ff)),
                desk_top: Hsla::from(gpui::rgba(0x5c3d2aff)),
                desk_base: Hsla::from(gpui::rgba(0x3d2b1fff)),
                desk_edge: Hsla::from(gpui::rgba(0x7a5238ff)),
                desk_light: Hsla::from(gpui::rgba(0xa06b48ff)),
                pc_bezel: Hsla::from(gpui::rgba(0x2a2a3eff)),
                pc_screen: Hsla::from(gpui::rgba(0x0d1117ff)),
                pc_glow: Hsla::from(gpui::rgba(0x1a2744ff)),
                plant_pot: Hsla::from(gpui::rgba(0x4a3520ff)),
                leaf1: Hsla::from(gpui::rgba(0x1a4a25ff)),
                leaf2: Hsla::from(gpui::rgba(0x1e5e2aff)),
                leaf3: Hsla::from(gpui::rgba(0x266b30ff)),
                sofa_base: Hsla::from(gpui::rgba(0x3d1525ff)),
                sofa_top: Hsla::from(gpui::rgba(0x5c1e38ff)),
                sofa_hl: Hsla::from(gpui::rgba(0x7a2a4aff)),
                shelf_base: Hsla::from(gpui::rgba(0x2a1a10ff)),
                shelf_wood: Hsla::from(gpui::rgba(0x3d2a18ff)),
                text: theme.foreground,
                text_dim: theme.muted_foreground,
                accent: theme.primary,
                green: Hsla::from(gpui::rgba(0x4ade80ff)),
                label_bg: Hsla {
                    a: 0.9,
                    ..darken(theme.background, 0.3)
                },
            }
        } else {
            // Light theme: brighter, softer tones
            Self {
                bg: darken(theme.background, 0.02),
                wall_front: Hsla::from(gpui::rgba(0x6f8799ff)),
                wall_shadow: Hsla::from(gpui::rgba(0x405768ff)),
                wall_highlight: Hsla::from(gpui::rgba(0x9fb7c8ff)),
                floor_dev: Hsla::from(gpui::rgba(0x9bbbd3ff)),
                floor_lounge: Hsla::from(gpui::rgba(0xb77a46ff)),
                floor_manager: Hsla::from(gpui::rgba(0x93b4ccff)),
                floor_kitchen: Hsla::from(gpui::rgba(0x8fa7b8ff)),
                floor_grid: Hsla::from(gpui::rgba(0x5f7e95ff)),
                floor_checker_light: Hsla::from(gpui::rgba(0xf2f4f6ff)),
                floor_checker_dark: Hsla::from(gpui::rgba(0x303844ff)),
                desk_top: Hsla::from(gpui::rgba(0xc9a882ff)),
                desk_base: Hsla::from(gpui::rgba(0xb89570ff)),
                desk_edge: Hsla::from(gpui::rgba(0xd4b896ff)),
                desk_light: Hsla::from(gpui::rgba(0xe0c9a8ff)),
                pc_bezel: Hsla::from(gpui::rgba(0xd0d0e0ff)),
                pc_screen: Hsla::from(gpui::rgba(0xe8eef4ff)),
                pc_glow: Hsla::from(gpui::rgba(0xdce8f4ff)),
                plant_pot: Hsla::from(gpui::rgba(0xc8a87cff)),
                leaf1: Hsla::from(gpui::rgba(0x5a9a65ff)),
                leaf2: Hsla::from(gpui::rgba(0x6aae6aff)),
                leaf3: Hsla::from(gpui::rgba(0x7ac07aff)),
                sofa_base: Hsla::from(gpui::rgba(0xd08090ff)),
                sofa_top: Hsla::from(gpui::rgba(0xe090a0ff)),
                sofa_hl: Hsla::from(gpui::rgba(0xf0a0b0ff)),
                shelf_base: Hsla::from(gpui::rgba(0xc8a87cff)),
                shelf_wood: Hsla::from(gpui::rgba(0xd4b896ff)),
                text: theme.foreground,
                text_dim: theme.muted_foreground,
                accent: theme.primary,
                green: Hsla::from(gpui::rgba(0x22c55eff)),
                label_bg: Hsla {
                    a: 0.92,
                    ..lighten(theme.background, 0.1)
                },
            }
        }
    }
}

// Color helpers

fn darken(c: Hsla, amount: f32) -> Hsla {
    Hsla {
        l: (c.l - amount).max(0.0),
        ..c
    }
}

fn lighten(c: Hsla, amount: f32) -> Hsla {
    Hsla {
        l: (c.l + amount).min(1.0),
        ..c
    }
}

fn with_alpha(c: Hsla, a: f32) -> Hsla {
    Hsla { a, ..c }
}

// Map generation

fn generate_map() -> [[u8; MAP_COLS]; MAP_ROWS] {
    let mut map = [[0u8; MAP_COLS]; MAP_ROWS];
    for y in 0..MAP_ROWS {
        for x in 0..MAP_COLS {
            if (y == 1 || y == MAP_ROWS - 2) && (x >= 1 && x <= MAP_COLS - 2) {
                map[y][x] = 1; // wall
            } else if (x == 1 || x == MAP_COLS - 2) && (y >= 1 && y <= MAP_ROWS - 2) {
                map[y][x] = 1; // wall
            } else if x == 12 || x == 22 {
                if y >= 6 && y <= 9 {
                    map[y][x] = if x < 15 { 2 } else { 3 }; // door
                } else if y > 1 && y < MAP_ROWS - 2 {
                    map[y][x] = 1; // inner wall
                }
            } else if x > 1 && x < 12 && y > 1 && y < MAP_ROWS - 2 {
                map[y][x] = 2; // dev room
            } else if x > 12 && x < 22 && y > 1 && y < MAP_ROWS - 2 {
                map[y][x] = if y > 9 { 4 } else { 3 }; // lounge / kitchen
            } else if x > 22 && x < MAP_COLS - 2 && y > 1 && y < MAP_ROWS - 2 {
                map[y][x] = 3; // manager
            }
        }
    }
    map
}

// Props

#[derive(Clone)]
struct Prop {
    kind: PropKind,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Clone, Copy)]
enum PropKind {
    Desk,
    Pc,
    Plant,
    Sofa,
    Table,
    Bookshelf,
    Frame,
}

const OFFICE_PROPS: &[Prop] = &[
    Prop {
        kind: PropKind::Bookshelf,
        x: 4.0,
        y: 2.0,
        w: 3.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Bookshelf,
        x: 8.0,
        y: 2.0,
        w: 3.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Plant,
        x: 2.0,
        y: 2.0,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Plant,
        x: 11.0,
        y: 2.0,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Plant,
        x: 2.0,
        y: 13.0,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Plant,
        x: 11.0,
        y: 13.0,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Desk,
        x: 3.0,
        y: 6.0,
        w: 3.0,
        h: 1.5,
    },
    Prop {
        kind: PropKind::Pc,
        x: 3.5,
        y: 5.5,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Pc,
        x: 5.0,
        y: 5.5,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Desk,
        x: 3.0,
        y: 7.5,
        w: 3.0,
        h: 1.5,
    },
    Prop {
        kind: PropKind::Pc,
        x: 3.5,
        y: 7.5,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Pc,
        x: 5.0,
        y: 7.5,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Desk,
        x: 8.0,
        y: 6.0,
        w: 3.0,
        h: 1.5,
    },
    Prop {
        kind: PropKind::Pc,
        x: 8.5,
        y: 5.5,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Pc,
        x: 10.0,
        y: 5.5,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Desk,
        x: 8.0,
        y: 7.5,
        w: 3.0,
        h: 1.5,
    },
    Prop {
        kind: PropKind::Pc,
        x: 8.5,
        y: 7.5,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Pc,
        x: 10.0,
        y: 7.5,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Plant,
        x: 13.0,
        y: 2.0,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Plant,
        x: 21.0,
        y: 2.0,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Frame,
        x: 15.0,
        y: 1.5,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Frame,
        x: 19.0,
        y: 1.5,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Sofa,
        x: 15.0,
        y: 6.0,
        w: 1.5,
        h: 3.0,
    },
    Prop {
        kind: PropKind::Sofa,
        x: 19.5,
        y: 6.0,
        w: 1.5,
        h: 3.0,
    },
    Prop {
        kind: PropKind::Table,
        x: 16.5,
        y: 6.5,
        w: 3.0,
        h: 2.0,
    },
    Prop {
        kind: PropKind::Bookshelf,
        x: 25.0,
        y: 2.0,
        w: 3.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Plant,
        x: 23.0,
        y: 13.0,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Plant,
        x: 29.0,
        y: 13.0,
        w: 1.0,
        h: 1.0,
    },
    Prop {
        kind: PropKind::Desk,
        x: 25.0,
        y: 7.0,
        w: 3.0,
        h: 1.5,
    },
    Prop {
        kind: PropKind::Pc,
        x: 26.0,
        y: 6.5,
        w: 1.0,
        h: 1.0,
    },
];

fn office_props() -> &'static [Prop] {
    OFFICE_PROPS
}

// Paint helpers

#[derive(Clone, Copy)]
struct OfficeLayout {
    origin: Point<Pixels>,
    tile: Pixels,
}

impl OfficeLayout {
    fn from_bounds(bounds: Bounds<Pixels>) -> Self {
        let tile_w = bounds.size.width / MAP_COLS as f32;
        let tile_h = bounds.size.height / MAP_ROWS as f32;
        let tile = if OFFICE_CANVAS_COVER {
            tile_w.max(tile_h)
        } else {
            tile_w.min(tile_h)
        };
        let total_w = tile * MAP_COLS as f32;
        let total_h = tile * MAP_ROWS as f32;

        Self {
            origin: point(
                bounds.origin.x + (bounds.size.width - total_w) / 2.0,
                bounds.origin.y + (bounds.size.height - total_h) / 2.0,
            ),
            tile,
        }
    }

    fn agent_center(&self, agent: &OfficeAgent) -> Point<Pixels> {
        point(
            self.origin.x + self.tile * agent.x + px(20.0),
            self.origin.y + self.tile * agent.y + px(20.0),
        )
    }

    fn agent_target_from_window_pos(&self, pos: Point<Pixels>) -> (f32, f32) {
        let pos_x: f32 = pos.x.into();
        let pos_y: f32 = pos.y.into();
        let origin_x: f32 = self.origin.x.into();
        let origin_y: f32 = self.origin.y.into();
        let tile: f32 = self.tile.into();

        (
            (pos_x - origin_x - 20.0) / tile,
            (pos_y - origin_y - 20.0) / tile,
        )
    }
}

fn clamp_agent_position(x: f32, y: f32, fallback: (f32, f32)) -> (f32, f32) {
    let x = x.clamp(2.0, (MAP_COLS - 3) as f32);
    let y = y.clamp(2.0, (MAP_ROWS - 3) as f32);
    let col = x.floor() as usize;
    let row = y.floor() as usize;
    let map = generate_map();

    if matches!(map[row][col], 2 | 3 | 4) {
        (x, y)
    } else {
        fallback
    }
}

fn paint_rect(window: &mut Window, x: Pixels, y: Pixels, w: Pixels, h: Pixels, color: Hsla) {
    let bounds = Bounds::from_corners(point(x, y), point(x + w, y + h));
    window.paint_quad(fill(bounds, color));
}

fn paint_rounded_rect(
    window: &mut Window,
    x: Pixels,
    y: Pixels,
    w: Pixels,
    h: Pixels,
    r: Pixels,
    color: Hsla,
) {
    let bounds = Bounds::from_corners(point(x, y), point(x + w, y + h));
    window.paint_quad(quad(
        bounds,
        r,
        color,
        Edges::default(),
        gpui::transparent_black(),
        BorderStyle::default(),
    ));
}

fn paint_text_centered(
    window: &mut Window,
    cx: &mut App,
    text: &str,
    cx_pos: Pixels,
    cy_pos: Pixels,
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
    let text_width = shaped.width;
    let origin = point(cx_pos - text_width / 2.0, cy_pos - font_size / 2.0);
    let _ = shaped.paint(origin, font_size, window, cx);
}

// Main paint function

#[derive(Clone, Copy)]
enum OfficeFloorPattern {
    Tile,
    Wood,
    Checker,
}

fn floor_cell_size(tile: Pixels, index: usize, extent: f32) -> Pixels {
    tile * (extent - index as f32).clamp(0.0, 1.0)
}

fn paint_floor_grid(
    window: &mut Window,
    rx: Pixels,
    ry: Pixels,
    rw: Pixels,
    rh: Pixels,
    tile: Pixels,
    cols: usize,
    rows: usize,
    color: Hsla,
) {
    let tile_f: f32 = tile.into();
    let line = px((tile_f * 0.035).clamp(1.0, 2.0));

    for col in 1..cols {
        paint_rect(window, rx + tile * col as f32, ry, line, rh, color);
    }
    for row in 1..rows {
        paint_rect(window, rx, ry + tile * row as f32, rw, line, color);
    }
    paint_rect(window, rx, ry, rw, line, with_alpha(color, 0.5));
    paint_rect(window, rx, ry + rh - line, rw, line, with_alpha(color, 0.5));
    paint_rect(window, rx, ry, line, rh, with_alpha(color, 0.5));
    paint_rect(window, rx + rw - line, ry, line, rh, with_alpha(color, 0.5));
}

fn paint_tiled_floor(
    window: &mut Window,
    rx: Pixels,
    ry: Pixels,
    rw: Pixels,
    rh: Pixels,
    tile: Pixels,
    cols: usize,
    rows: usize,
    extent_w: f32,
    extent_h: f32,
    palette: &OfficePalette,
    color: Hsla,
) {
    paint_rect(window, rx, ry, rw, rh, color);

    for row in 0..rows {
        for col in 0..cols {
            if (row + col) % 2 == 0 {
                let cell_w = floor_cell_size(tile, col, extent_w);
                let cell_h = floor_cell_size(tile, row, extent_h);
                paint_rect(
                    window,
                    rx + tile * col as f32,
                    ry + tile * row as f32,
                    cell_w,
                    cell_h,
                    with_alpha(lighten(color, 0.04), 0.18),
                );
            }
        }
    }

    paint_floor_grid(
        window,
        rx,
        ry,
        rw,
        rh,
        tile,
        cols,
        rows,
        with_alpha(palette.floor_grid, 0.6),
    );
}

fn paint_wood_floor(
    window: &mut Window,
    rx: Pixels,
    ry: Pixels,
    rw: Pixels,
    rh: Pixels,
    tile: Pixels,
    cols: usize,
    rows: usize,
    extent_w: f32,
    extent_h: f32,
    color: Hsla,
) {
    paint_rect(window, rx, ry, rw, rh, color);

    for row in 0..rows {
        for col in 0..cols {
            let cell_w = floor_cell_size(tile, col, extent_w);
            let cell_h = floor_cell_size(tile, row, extent_h);
            let shade = if (row + col) % 2 == 0 {
                lighten(color, 0.035)
            } else {
                darken(color, 0.035)
            };
            paint_rect(
                window,
                rx + tile * col as f32,
                ry + tile * row as f32,
                cell_w,
                cell_h,
                with_alpha(shade, 0.35),
            );
        }
    }

    paint_floor_grid(
        window,
        rx,
        ry,
        rw,
        rh,
        tile,
        cols,
        rows,
        with_alpha(darken(color, 0.16), 0.82),
    );

    for row in 0..rows {
        let seam_y = ry + tile * row as f32 + tile * 0.5;
        paint_rect(
            window,
            rx,
            seam_y,
            rw,
            px(1.0),
            with_alpha(darken(color, 0.12), 0.45),
        );
    }
}

fn paint_checker_floor(
    window: &mut Window,
    rx: Pixels,
    ry: Pixels,
    rw: Pixels,
    rh: Pixels,
    tile: Pixels,
    cols: usize,
    rows: usize,
    extent_w: f32,
    extent_h: f32,
    palette: &OfficePalette,
) {
    paint_rect(window, rx, ry, rw, rh, palette.floor_kitchen);

    for row in 0..rows {
        for col in 0..cols {
            let cell_w = floor_cell_size(tile, col, extent_w);
            let cell_h = floor_cell_size(tile, row, extent_h);
            let color = if (row + col) % 2 == 0 {
                palette.floor_checker_light
            } else {
                palette.floor_checker_dark
            };
            paint_rect(
                window,
                rx + tile * col as f32,
                ry + tile * row as f32,
                cell_w,
                cell_h,
                color,
            );
        }
    }

    paint_floor_grid(
        window,
        rx,
        ry,
        rw,
        rh,
        tile,
        cols,
        rows,
        with_alpha(palette.wall_shadow, 0.5),
    );
}

fn paint_room_floor(
    window: &mut Window,
    ox: Pixels,
    oy: Pixels,
    tile: Pixels,
    palette: &OfficePalette,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Hsla,
    pattern: OfficeFloorPattern,
) {
    let rx = ox + tile * x;
    let ry = oy + tile * y;
    let rw = tile * w;
    let rh = tile * h;
    let cols = w.ceil() as usize;
    let rows = h.ceil() as usize;

    match pattern {
        OfficeFloorPattern::Tile => paint_tiled_floor(
            window, rx, ry, rw, rh, tile, cols, rows, w, h, palette, color,
        ),
        OfficeFloorPattern::Wood => {
            paint_wood_floor(window, rx, ry, rw, rh, tile, cols, rows, w, h, color)
        }
        OfficeFloorPattern::Checker => {
            paint_checker_floor(window, rx, ry, rw, rh, tile, cols, rows, w, h, palette)
        }
    }
}

fn paint_wall_segment(
    window: &mut Window,
    x: Pixels,
    y: Pixels,
    w: Pixels,
    h: Pixels,
    palette: &OfficePalette,
) {
    paint_rect(window, x, y, w, h, palette.wall_front);

    let w_f: f32 = w.into();
    let h_f: f32 = h.into();
    let trim = px((w_f.min(h_f) * 0.14).clamp(2.0, 5.0));

    paint_rect(
        window,
        x,
        y,
        w,
        trim,
        with_alpha(palette.wall_highlight, 0.62),
    );
    paint_rect(
        window,
        x,
        y,
        trim,
        h,
        with_alpha(palette.wall_highlight, 0.38),
    );
    paint_rect(
        window,
        x,
        y + h - trim,
        w,
        trim,
        with_alpha(palette.wall_shadow, 0.82),
    );
    paint_rect(
        window,
        x + w - trim,
        y,
        trim,
        h,
        with_alpha(palette.wall_shadow, 0.65),
    );

    for stripe in 1..4 {
        let stripe_y = y + h * (stripe as f32 / 4.0);
        paint_rect(
            window,
            x + trim,
            stripe_y,
            w - trim * 2.0,
            px(1.0),
            with_alpha(palette.wall_highlight, 0.18),
        );
    }
}

fn paint_room_shell(
    window: &mut Window,
    ox: Pixels,
    oy: Pixels,
    tile: Pixels,
    palette: &OfficePalette,
) {
    let tile_f: f32 = tile.into();
    let wall = px((tile_f * 0.58).clamp(14.0, 26.0));
    let full_x = ox + tile;
    let full_y = oy + tile;
    let full_w = tile * 30.0;
    let full_h = tile * 14.0;

    paint_room_floor(
        window,
        ox,
        oy,
        tile,
        palette,
        1.0,
        1.0,
        11.0,
        14.0,
        palette.floor_dev,
        OfficeFloorPattern::Tile,
    );
    paint_room_floor(
        window,
        ox,
        oy,
        tile,
        palette,
        12.0,
        1.0,
        10.0,
        8.5,
        palette.floor_lounge,
        OfficeFloorPattern::Wood,
    );
    paint_room_floor(
        window,
        ox,
        oy,
        tile,
        palette,
        12.0,
        9.5,
        10.0,
        5.5,
        palette.floor_kitchen,
        OfficeFloorPattern::Checker,
    );
    paint_room_floor(
        window,
        ox,
        oy,
        tile,
        palette,
        22.0,
        1.0,
        9.0,
        14.0,
        palette.floor_manager,
        OfficeFloorPattern::Tile,
    );

    paint_wall_segment(window, full_x, full_y, full_w, wall, palette);
    paint_wall_segment(
        window,
        full_x,
        full_y + full_h - wall,
        full_w,
        wall,
        palette,
    );
    paint_wall_segment(window, full_x, full_y, wall, full_h, palette);
    paint_wall_segment(
        window,
        full_x + full_w - wall,
        full_y,
        wall,
        full_h,
        palette,
    );

    for divider_x in [12.0, 22.0] {
        let x = ox + tile * divider_x - wall / 2.0;
        paint_wall_segment(window, x, oy + tile * 2.0, wall, tile * 4.0, palette);
        paint_wall_segment(window, x, oy + tile * 10.0, wall, tile * 4.0, palette);
        paint_rect(
            window,
            x - px(2.0),
            oy + tile * 6.0,
            wall + px(4.0),
            px(3.0),
            with_alpha(palette.wall_highlight, 0.72),
        );
        paint_rect(
            window,
            x - px(2.0),
            oy + tile * 9.0,
            wall + px(4.0),
            px(3.0),
            with_alpha(palette.wall_shadow, 0.72),
        );
    }
}

fn paint_office(
    bounds: Bounds<Pixels>,
    state: &OfficeState,
    reduce_text_paint: bool,
    window: &mut Window,
    cx: &mut App,
) {
    let palette = OfficePalette::from_theme(cx.theme());
    let layout = OfficeLayout::from_bounds(bounds);
    let tile = layout.tile;
    let ox = layout.origin.x;
    let oy = layout.origin.y;

    // Background
    window.paint_quad(fill(bounds, palette.bg));
    paint_room_shell(window, ox, oy, tile, &palette);

    if !reduce_text_paint {
        // Room labels
        paint_text_centered(
            window,
            cx,
            "DEV ROOM",
            ox + tile * 6.5,
            oy + tile * 3.5,
            px(9.0),
            with_alpha(palette.text_dim, 0.32),
        );
        paint_text_centered(
            window,
            cx,
            "COORDINATION",
            ox + tile * 17.0,
            oy + tile * 3.5,
            px(9.0),
            with_alpha(palette.text_dim, 0.32),
        );
        paint_text_centered(
            window,
            cx,
            "WAITING",
            ox + tile * 17.2,
            oy + tile * 11.2,
            px(9.0),
            with_alpha(palette.text_dim, 0.32),
        );
        paint_text_centered(
            window,
            cx,
            "MANAGER",
            ox + tile * 27.0,
            oy + tile * 3.5,
            px(9.0),
            with_alpha(palette.text_dim, 0.32),
        );
    }

    // Props
    for p in office_props() {
        let ppx = ox + tile * p.x;
        let ppy = oy + tile * p.y;
        let ppw = tile * p.w;
        let pph = tile * p.h;

        match p.kind {
            PropKind::Desk => {
                // Shadow
                paint_rect(
                    window,
                    ppx + px(3.0),
                    ppy + pph,
                    ppw - px(3.0),
                    px(6.0),
                    with_alpha(palette.bg, 0.4),
                );
                // Body
                paint_rect(window, ppx, ppy, ppw, pph, palette.desk_base);
                paint_rect(window, ppx, ppy, ppw, pph * 0.7, palette.desk_top);
                // Highlight
                paint_rect(
                    window,
                    ppx + px(3.0),
                    ppy + px(2.0),
                    ppw - px(6.0),
                    px(3.0),
                    palette.desk_light,
                );
                // Edge
                paint_rect(
                    window,
                    ppx,
                    ppy + pph * 0.7,
                    ppw,
                    px(3.0),
                    palette.desk_edge,
                );
                // Legs
                paint_rect(
                    window,
                    ppx + px(3.0),
                    ppy + pph - px(8.0),
                    px(5.0),
                    px(8.0),
                    palette.desk_base,
                );
                paint_rect(
                    window,
                    ppx + ppw - px(8.0),
                    ppy + pph - px(8.0),
                    px(5.0),
                    px(8.0),
                    palette.desk_base,
                );
            }
            PropKind::Pc => {
                // Stand
                paint_rect(
                    window,
                    ppx + px(18.0),
                    ppy + px(28.0),
                    px(4.0),
                    px(6.0),
                    palette.pc_bezel,
                );
                paint_rect(
                    window,
                    ppx + px(14.0),
                    ppy + px(33.0),
                    px(12.0),
                    px(3.0),
                    palette.pc_bezel,
                );
                // Bezel
                paint_rect(
                    window,
                    ppx + px(3.0),
                    ppy + px(5.0),
                    px(28.0),
                    px(20.0),
                    palette.pc_bezel,
                );
                // Screen
                paint_rect(
                    window,
                    ppx + px(5.0),
                    ppy + px(7.0),
                    px(24.0),
                    px(16.0),
                    palette.pc_screen,
                );
                paint_rect(
                    window,
                    ppx + px(6.0),
                    ppy + px(8.0),
                    px(22.0),
                    px(14.0),
                    with_alpha(palette.pc_glow, 0.7),
                );
                // Code lines
                let code_colors = [
                    Hsla::from(gpui::rgba(0x4a9effff)),
                    palette.accent,
                    palette.green,
                    Hsla::from(gpui::rgba(0xfbbf24ff)),
                    Hsla::from(gpui::rgba(0xe879f9ff)),
                ];
                for (i, cc) in code_colors.iter().enumerate() {
                    paint_rect(
                        window,
                        ppx + px(6.0),
                        ppy + px(9.0 + i as f32 * 3.0),
                        px(6.0 + i as f32 * 3.0),
                        px(1.5),
                        with_alpha(*cc, 0.7),
                    );
                }
                // Glare
                paint_rect(
                    window,
                    ppx + px(5.0),
                    ppy + px(7.0),
                    px(12.0),
                    px(5.0),
                    with_alpha(palette.text, 0.03),
                );
                // Keyboard
                paint_rect(
                    window,
                    ppx + px(6.0),
                    ppy + px(28.0),
                    px(20.0),
                    px(4.0),
                    palette.pc_bezel,
                );
            }
            PropKind::Plant => {
                let cx_p = ppx + px(20.0);
                let pot_y = ppy + px(22.0);
                // Pot shadow
                paint_rect(
                    window,
                    cx_p - px(11.0),
                    pot_y + px(12.0),
                    px(22.0),
                    px(8.0),
                    with_alpha(palette.bg, 0.3),
                );
                // Pot
                paint_rect(
                    window,
                    cx_p - px(9.0),
                    pot_y,
                    px(18.0),
                    px(16.0),
                    palette.plant_pot,
                );
                // Pot rim
                paint_rect(
                    window,
                    cx_p - px(10.0),
                    pot_y - px(3.0),
                    px(20.0),
                    px(4.0),
                    lighten(palette.plant_pot, 0.05),
                );
                // Leaves (approximated as rounded rects)
                paint_rounded_rect(
                    window,
                    cx_p - px(12.0),
                    pot_y - px(20.0),
                    px(24.0),
                    px(16.0),
                    px(8.0),
                    palette.leaf1,
                );
                paint_rounded_rect(
                    window,
                    cx_p - px(8.0),
                    pot_y - px(16.0),
                    px(16.0),
                    px(12.0),
                    px(6.0),
                    palette.leaf2,
                );
                paint_rounded_rect(
                    window,
                    cx_p - px(6.0),
                    pot_y - px(22.0),
                    px(12.0),
                    px(10.0),
                    px(5.0),
                    palette.leaf3,
                );
            }
            PropKind::Sofa => {
                // Shadow
                paint_rect(
                    window,
                    ppx + px(3.0),
                    ppy + pph,
                    ppw - px(3.0),
                    px(5.0),
                    with_alpha(palette.bg, 0.35),
                );
                // Base
                paint_rect(window, ppx, ppy, ppw, pph, palette.sofa_base);
                // Cushion
                paint_rect(
                    window,
                    ppx + px(3.0),
                    ppy + px(3.0),
                    ppw - px(6.0),
                    pph - px(12.0),
                    palette.sofa_top,
                );
                // Arms
                paint_rect(window, ppx, ppy, px(7.0), pph - px(5.0), palette.sofa_hl);
                paint_rect(
                    window,
                    ppx + ppw - px(7.0),
                    ppy,
                    px(7.0),
                    pph - px(5.0),
                    palette.sofa_hl,
                );
                // Highlight
                paint_rect(
                    window,
                    ppx + px(3.0),
                    ppy + px(3.0),
                    ppw - px(6.0),
                    px(3.0),
                    with_alpha(palette.text, 0.06),
                );
            }
            PropKind::Table => {
                // Shadow
                paint_rect(
                    window,
                    ppx + px(5.0),
                    ppy + pph,
                    ppw - px(5.0),
                    px(5.0),
                    with_alpha(palette.bg, 0.35),
                );
                // Base
                let table_base = Hsla::from(gpui::rgba(0x1a2a3aff));
                paint_rect(window, ppx, ppy, ppw, pph, table_base);
                // Glass top
                let table_glass = Hsla::from(gpui::rgba(0x0d2a3aff));
                paint_rect(
                    window,
                    ppx + px(5.0),
                    ppy + px(5.0),
                    ppw - px(10.0),
                    pph - px(10.0),
                    table_glass,
                );
                // Reflection
                paint_rect(
                    window,
                    ppx + px(5.0),
                    ppy + px(5.0),
                    ppw - px(10.0),
                    (pph - px(10.0)) / 2.0,
                    with_alpha(palette.text, 0.02),
                );
                // Objects on table
                paint_rect(
                    window,
                    ppx + px(12.0),
                    ppy + px(12.0),
                    px(7.0),
                    px(7.0),
                    palette.text,
                );
            }
            PropKind::Bookshelf => {
                paint_rect(window, ppx, ppy, ppw, pph, palette.shelf_base);
                paint_rect(window, ppx, ppy, ppw, pph - px(8.0), palette.shelf_wood);
                paint_rect(
                    window,
                    ppx + ppw / 2.0 - px(1.0),
                    ppy,
                    px(2.0),
                    pph - px(8.0),
                    palette.shelf_base,
                );
                // Books
                let book_colors = [
                    Hsla::from(gpui::rgba(0xf472b6ff)),
                    Hsla::from(gpui::rgba(0x60a5faff)),
                    palette.green,
                    Hsla::from(gpui::rgba(0xfbbf24ff)),
                    Hsla::from(gpui::rgba(0xa78bfaff)),
                    Hsla::from(gpui::rgba(0xfb923cff)),
                ];
                let mut bx = ppx + px(3.0);
                for i in 0..9 {
                    let bw = px(7.0 + (i % 3) as f32);
                    paint_rect(
                        window,
                        bx,
                        ppy + px(3.0),
                        bw,
                        pph - px(14.0),
                        with_alpha(book_colors[i % book_colors.len()], 0.85),
                    );
                    // Spine highlight
                    paint_rect(
                        window,
                        bx,
                        ppy + px(3.0),
                        px(2.0),
                        pph - px(14.0),
                        with_alpha(palette.text, 0.15),
                    );
                    bx = bx + bw + px(2.0);
                    if bx > ppx + ppw - px(10.0) {
                        bx = ppx + px(3.0);
                    }
                }
            }
            PropKind::Frame => {
                let frame_bg = Hsla::from(gpui::rgba(0x3a2a18ff));
                paint_rect(window, ppx, ppy, px(32.0), px(20.0), frame_bg);
                // Inner gradient approximation
                paint_rect(
                    window,
                    ppx + px(3.0),
                    ppy + px(3.0),
                    px(26.0),
                    px(7.0),
                    Hsla::from(gpui::rgba(0x1a1a3aff)),
                );
                paint_rect(
                    window,
                    ppx + px(3.0),
                    ppy + px(10.0),
                    px(26.0),
                    px(7.0),
                    Hsla::from(gpui::rgba(0x3a1a2aff)),
                );
            }
        }
    }

    // Agents
    let time = state.elapsed_secs();
    for agent in &state.agents {
        let apx = ox + tile * agent.x;
        let apy = oy + tile * agent.y;

        let bob = match agent.status.as_str() {
            "coding" => (time * 10.0).sin() * 2.0,
            "communicating" => (time * 3.0).sin() * 3.0,
            "idle" => (time * 1.0).sin() * 1.0,
            _ => (time * 5.0).sin() * 1.5,
        };

        let bx = apx + px(20.0);
        let by = apy + px(20.0) + px(bob);
        let sc = status_color(&agent.status);

        // Shadow
        paint_rect(
            window,
            bx - px(12.0),
            apy + px(34.0),
            px(24.0),
            px(8.0),
            with_alpha(palette.bg, 0.3),
        );

        // Glow halo
        paint_rounded_rect(
            window,
            bx - px(17.0),
            by - px(15.0),
            px(34.0),
            px(42.0),
            px(17.0),
            with_alpha(sc, 0.08),
        );

        // Legs
        paint_rect(
            window,
            bx - px(7.0),
            by + px(10.0),
            px(5.0),
            px(11.0),
            palette.pc_bezel,
        );
        paint_rect(
            window,
            bx + px(2.0),
            by + px(10.0),
            px(5.0),
            px(11.0),
            palette.pc_bezel,
        );
        // Shoes
        paint_rect(
            window,
            bx - px(9.0),
            by + px(19.0),
            px(7.0),
            px(4.0),
            agent.color,
        );
        paint_rect(
            window,
            bx + px(2.0),
            by + px(19.0),
            px(7.0),
            px(4.0),
            agent.color,
        );
        // Body
        paint_rect(
            window,
            bx - px(9.0),
            by - px(2.0),
            px(18.0),
            px(12.0),
            with_alpha(agent.color, 0.8),
        );
        // Body highlight
        paint_rect(
            window,
            bx - px(7.0),
            by,
            px(14.0),
            px(3.0),
            with_alpha(palette.text, 0.1),
        );
        // Arms
        paint_rect(
            window,
            bx - px(13.0),
            by,
            px(5.0),
            px(9.0),
            with_alpha(agent.color, 0.65),
        );
        paint_rect(
            window,
            bx + px(8.0),
            by,
            px(5.0),
            px(9.0),
            with_alpha(agent.color, 0.65),
        );
        // Hands
        let skin = Hsla::from(gpui::rgba(0xfde8d0ff));
        paint_rect(window, bx - px(13.0), by + px(8.0), px(5.0), px(4.0), skin);
        paint_rect(window, bx + px(8.0), by + px(8.0), px(5.0), px(4.0), skin);
        // Neck
        paint_rect(window, bx - px(3.0), by - px(6.0), px(6.0), px(5.0), skin);
        // Head
        paint_rect(
            window,
            bx - px(9.0),
            by - px(20.0),
            px(18.0),
            px(15.0),
            skin,
        );
        // Hair
        paint_rect(
            window,
            bx - px(11.0),
            by - px(24.0),
            px(22.0),
            px(10.0),
            agent.color,
        );
        paint_rect(
            window,
            bx - px(11.0),
            by - px(14.0),
            px(4.0),
            px(7.0),
            agent.color,
        );
        paint_rect(
            window,
            bx + px(7.0),
            by - px(14.0),
            px(4.0),
            px(7.0),
            agent.color,
        );
        // Eyes
        let eye_color = Hsla::from(gpui::rgba(0x0a0a14ff));
        paint_rect(
            window,
            bx - px(6.0),
            by - px(16.0),
            px(4.0),
            px(4.0),
            eye_color,
        );
        paint_rect(
            window,
            bx + px(2.0),
            by - px(16.0),
            px(4.0),
            px(4.0),
            eye_color,
        );
        // Eye glints
        paint_rect(
            window,
            bx - px(5.0),
            by - px(15.0),
            px(2.0),
            px(2.0),
            with_alpha(palette.text, 0.7),
        );
        paint_rect(
            window,
            bx + px(3.0),
            by - px(15.0),
            px(2.0),
            px(2.0),
            with_alpha(palette.text, 0.7),
        );
        // Mouth
        paint_rect(
            window,
            bx - px(2.0),
            by - px(10.0),
            px(4.0),
            px(2.0),
            Hsla::from(gpui::rgba(0xc0706aff)),
        );

        if !reduce_text_paint {
            // Name tag
            let tag_y = apy - px(6.0) + px(bob * 0.4);
            let name_text = &agent.name;
            let tw = px(name_text.chars().count().max(6) as f32 * 5.5 + 14.0);
            // Tag background
            paint_rounded_rect(
                window,
                bx - tw / 2.0,
                tag_y - px(13.0),
                tw,
                px(14.0),
                px(3.0),
                palette.label_bg,
            );
            // Tag accent bar
            paint_rect(
                window,
                bx - tw / 2.0,
                tag_y - px(13.0),
                px(3.0),
                px(14.0),
                agent.color,
            );
            // Name text
            paint_text_centered(
                window,
                cx,
                name_text,
                bx + px(1.0),
                tag_y - px(6.0),
                px(9.0),
                palette.text,
            );

            // Status badge
            let status_text = &agent.status;
            let sw = px(status_text.chars().count().max(4) as f32 * 5.0 + 10.0);
            paint_rounded_rect(
                window,
                bx - sw / 2.0,
                tag_y + px(3.0),
                sw,
                px(12.0),
                px(5.0),
                palette.label_bg,
            );
            paint_text_centered(window, cx, status_text, bx, tag_y + px(9.0), px(9.0), sc);

            // Message bubble
            if let Some(msg) = &agent.message {
                let mw = px(msg.chars().count().max(5) as f32 * 5.5 + 18.0);
                paint_rounded_rect(
                    window,
                    bx - mw / 2.0,
                    tag_y - px(36.0),
                    mw,
                    px(22.0),
                    px(6.0),
                    palette.label_bg,
                );
                // Pointer triangle approximation
                paint_rect(
                    window,
                    bx - px(3.0),
                    tag_y - px(14.0),
                    px(6.0),
                    px(6.0),
                    palette.label_bg,
                );
                // Message text
                paint_text_centered(window, cx, msg, bx, tag_y - px(25.0), px(9.0), palette.text);
            }
        }
    }
}

fn status_color(status: &str) -> Hsla {
    match status.to_lowercase().as_str() {
        "coding" => Hsla::from(gpui::rgba(0x4ade80ff)),
        "reviewing" => Hsla::from(gpui::rgba(0xfbbf24ff)),
        "communicating" => Hsla::from(gpui::rgba(0x60a5faff)),
        "failed" => Hsla::from(gpui::rgba(0xf87171ff)),
        "planning" => Hsla::from(gpui::rgba(0xa78bfaff)),
        "debugging" => Hsla::from(gpui::rgba(0xfb923cff)),
        _ => Hsla::from(gpui::rgba(0x6b6890ff)),
    }
}

// Public render function

fn strip_tool_calls(raw: &str) -> String {
    let mut out = String::new();
    let mut i = 0usize;
    loop {
        let Some(start_rel) = raw[i..].find("<tool_call") else {
            out.push_str(&raw[i..]);
            break;
        };
        let start = i + start_rel;
        out.push_str(&raw[i..start]);
        let after = start + "<tool_call>".len();
        if raw[start..].starts_with("<tool_call>") {
            if let Some(end_rel) = raw[after..].find("</tool_call>") {
                i = after + end_rel + "</tool_call>".len();
                continue;
            }
        }
        out.push_str(&raw[start..]);
        break;
    }
    out
}

fn strip_stream_display_markers(raw: &str) -> String {
    let without_tools = strip_tool_calls(raw);
    STREAM_DISPLAY_MARKERS
        .iter()
        .fold(without_tools, |text, marker| text.replace(marker, ""))
}

pub(crate) fn office_visible_message_text(value: &str, max_chars: usize) -> Option<String> {
    let normalized = strip_stream_display_markers(value).replace('\r', "");
    let collapsed = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.trim().is_empty() {
        return None;
    }
    let mut out = String::new();
    for (idx, ch) in collapsed.chars().enumerate() {
        if idx >= max_chars {
            out.push_str("...");
            return Some(out);
        }
        out.push(ch);
    }
    Some(out)
}

fn office_plain_text(value: &str, max_chars: usize) -> String {
    let normalized = strip_stream_display_markers(value).replace('\r', "");
    let mut out = String::new();
    let mut chars = normalized.chars();
    let mut run_len = 0usize;
    for _ in 0..max_chars {
        let Some(ch) = chars.next() else {
            return out;
        };
        if ch.is_whitespace() {
            run_len = 0;
        } else {
            if run_len >= 48 {
                out.push(' ');
                run_len = 0;
            }
            run_len += 1;
        }
        out.push(ch);
    }
    if chars.next().is_some() {
        out.push_str("...");
    }
    out
}

impl super::TeamWorkspacePanel {
    fn render_office_chat_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let session_id = self.selected_session_id.clone();
        let messages = session_id
            .as_ref()
            .and_then(|id| self.chat_histories.get(id))
            .cloned()
            .unwrap_or_default();
        let display_len = messages.len();

        let message_area = if display_len == 0 {
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(12.0))
                .text_color(theme.muted_foreground)
                .child("No chat messages yet")
                .into_any_element()
        } else {
            let mut message_list = v_flex().w_full().gap(px(8.0)).p(px(12.0));
            let start = messages.len().saturating_sub(40);
            for (idx, msg) in messages.iter().enumerate().skip(start) {
                let body_text = office_plain_text(msg.content.as_ref(), 1400);
                if body_text.trim().is_empty() {
                    continue;
                }
                let is_user = msg.role == "user";
                let author = if is_user {
                    "You".to_string()
                } else {
                    msg.agent_name
                        .as_ref()
                        .map(|name| name.to_string())
                        .unwrap_or_else(|| "Agent".to_string())
                };
                let accent = if is_user {
                    theme.primary
                } else {
                    gpui::Hsla::from(gpui::rgba(0x8b5cf6ff))
                };
                message_list = message_list.child(
                    div()
                        .id(("office-msg", idx))
                        .w_full()
                        .min_w_0()
                        .overflow_hidden()
                        .rounded(px(8.0))
                        .border_1()
                        .border_color(accent.opacity(0.18))
                        .bg(if is_user {
                            theme.primary.opacity(0.06)
                        } else {
                            theme.secondary.opacity(0.35)
                        })
                        .p(px(10.0))
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .mb(px(6.0))
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(accent)
                                        .child(author),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(theme.muted_foreground)
                                        .child(msg.role.to_string()),
                                ),
                        )
                        .child(
                            div()
                                .w_full()
                                .min_w_0()
                                .overflow_hidden()
                                .text_size(px(12.0))
                                .line_height(gpui::relative(1.35))
                                .whitespace_normal()
                                .text_color(theme.foreground)
                                .child(body_text),
                        ),
                );
            }

            div()
                .id("office-chat-scroll")
                .size_full()
                .overflow_y_scrollbar()
                .child(message_list)
                .into_any_element()
        };

        let input_row = h_flex()
            .w_full()
            .h(px(56.0))
            .flex_shrink_0()
            .overflow_hidden()
            .gap(px(8.0))
            .items_center()
            .p(px(10.0))
            .border_t_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h(px(36.0))
                    .max_h(px(36.0))
                    .overflow_hidden()
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .child(
                        Input::new(&self.chat_input_state)
                            .appearance(false)
                            .h(px(36.0))
                            .w_full()
                            .min_w_0()
                            .overflow_hidden(),
                    ),
            )
            .child(
                Button::new("office-send-chat")
                    .small()
                    .icon(if self.is_generating {
                        IconName::Close
                    } else {
                        IconName::ArrowUp
                    })
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.handle_send_chat(window, cx);
                    })),
            );

        v_flex()
            .w_full()
            .flex_shrink_0()
            .h(px(300.0))
            .min_h_0()
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.background)
            .child(
                h_flex()
                    .h(px(44.0))
                    .w_full()
                    .px(px(12.0))
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme.foreground)
                            .child("Office Chat"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.muted_foreground)
                            .child(format!("{} messages", display_len)),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(message_area),
            )
            .child(input_row)
    }

    pub(crate) fn render_office_view(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let view = cx.entity().clone();

        // Gather active agents for the sidebar
        let mut active_agents: Vec<(String, String, String, usize, Hsla)> = Vec::new();
        for agent in &self.office_state.agents {
            active_agents.push((
                agent.id.clone(),
                agent.name.clone(),
                agent.status.clone(),
                agent.task_count,
                agent.color,
            ));
        }
        let agent_count = active_agents.len();
        let selected_agent = self.selected_office_agent_id.as_ref().and_then(|agent_id| {
            self.office_state
                .agents
                .iter()
                .find(|agent| &agent.id == agent_id)
                .cloned()
        });
        let selected_agent_run = selected_agent.as_ref().and_then(|agent| {
            let instance_id = self.selected_instance_id.as_ref()?;
            let db = crate::AppState::global(cx).db.clone();
            let task = db
                .list_tasks_for_instance(instance_id)
                .ok()?
                .into_iter()
                .filter(|task| {
                    task.assignee_id.as_deref() == Some(agent.id.as_str())
                        && task.run_id.is_some()
                        && !matches!(task.status.as_str(), "completed" | "failed" | "cancelled")
                })
                .max_by(|left, right| left.updated_at.cmp(&right.updated_at))?;
            let run_id = task.run_id.clone()?;
            let run = db.get_orchestration_run(&run_id).ok().flatten()?;
            Some((task, run))
        });

        // Avoid competing with text input while the inline office chat is active.
        let quick_chat_open = self.office_quick_chat_agent_id.is_some();
        if !quick_chat_open {
            self.office_state.tick();
        }
        let should_continue_office_animation =
            !quick_chat_open && self.office_state.needs_animation_tick();
        let office_animation_delay = if self.office_state.has_active_motion() {
            std::time::Duration::from_millis(50)
        } else {
            std::time::Duration::from_millis(250)
        };

        // Build sidebar
        let sidebar = {
            let mut agent_list = v_flex().w_full().gap(px(6.0)).p(px(8.0));
            for (agent_id, name, status, task_count, color) in &active_agents {
                let sc = status_color(status);
                let is_selected = self.selected_office_agent_id.as_deref() == Some(agent_id);
                let status_label = if *task_count > 0 {
                    format!("{} tasks", task_count)
                } else {
                    status.clone()
                };
                let initials: String = name.chars().take(2).collect::<String>().to_uppercase();
                let selected_agent_id = agent_id.clone();
                agent_list = agent_list.child(
                    h_flex()
                        .id(gpui::ElementId::Name(
                            format!("office-agent-row-{}", agent_id).into(),
                        ))
                        .w_full()
                        .px(px(12.0))
                        .py(px(6.0))
                        .gap(px(9.0))
                        .items_center()
                        .rounded(px(8.0))
                        .border_1()
                        .border_color(if is_selected {
                            theme.primary.opacity(0.32)
                        } else {
                            theme.border.opacity(0.55)
                        })
                        .bg(if is_selected {
                            theme.primary.opacity(0.12)
                        } else {
                            theme.background.opacity(0.18)
                        })
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.primary.opacity(0.07)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.selected_office_agent_id = Some(selected_agent_id.clone());
                            cx.notify();
                        }))
                        .child(
                            div()
                                .w(px(26.0))
                                .h(px(26.0))
                                .rounded(px(6.0))
                                .bg(color.opacity(0.15))
                                .border_1()
                                .border_color(color.opacity(0.3))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(*color)
                                .text_size(px(11.0))
                                .font_weight(gpui::FontWeight::BOLD)
                                .child(initials),
                        )
                        .child(
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(theme.foreground)
                                        .overflow_hidden()
                                        .child(name.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .mt(px(2.0))
                                        .px(px(6.0))
                                        .py(px(1.0))
                                        .rounded(px(4.0))
                                        .border_1()
                                        .border_color(sc.opacity(0.3))
                                        .text_color(sc)
                                        .child(status_label),
                                ),
                        ),
                );
            }

            let agent_inspector = selected_agent.map(|agent| {
                let status = status_color(&agent.status);
                let has_run = selected_agent_run.is_some();
                let run_detail = selected_agent_run.clone().map(|(task, run)| {
                    let open_run_id = run.id.clone();
                    let run_label = run.id.chars().take(8).collect::<String>();
                    v_flex()
                        .gap(px(6.0))
                        .pt(px(8.0))
                        .border_t_1()
                        .border_color(theme.border)
                        .child(
                            h_flex()
                                .justify_between()
                                .text_size(px(10.0))
                                .child(
                                    div()
                                        .text_color(theme.muted_foreground)
                                        .child("Current run"),
                                )
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(theme.foreground)
                                        .child(run_label),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(theme.muted_foreground)
                                .line_height(gpui::relative(1.35))
                                .child(format!("{} | {}", run.status, task.status)),
                        )
                        .child(
                            Button::new(gpui::SharedString::from(format!(
                                "office-open-run-{}",
                                open_run_id
                            )))
                            .small()
                            .label("Open run")
                            .on_click(cx.listener(
                                move |_this, _, _, cx| {
                                    let state = crate::AppState::global(cx);
                                    let _ = state
                                        .db
                                        .set_setting("orchestration_selected_run_id", &open_run_id);
                                    let selected = state.selected_orchestration_run_id.clone();
                                    let active_panel = state.active_panel.clone();
                                    let selected_run_id = open_run_id.clone();
                                    selected.update(cx, move |current, cx| {
                                        *current = Some(selected_run_id);
                                        cx.notify();
                                    });
                                    active_panel.update(cx, |page, cx| {
                                        *page = "orchestration".to_string();
                                        cx.notify();
                                    });
                                },
                            )),
                        )
                        .into_any_element()
                });
                v_flex()
                    .gap(px(6.0))
                    .p(px(10.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded(px(10.0))
                    .bg(theme.secondary)
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme.muted_foreground)
                            .child("SELECTED AGENT"),
                    )
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme.foreground)
                            .child(agent.name),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .text_size(px(10.0))
                            .child(div().text_color(theme.muted_foreground).child("Status"))
                            .child(div().text_color(status).child(agent.status)),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .text_size(px(10.0))
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child("Active tasks"),
                            )
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.foreground)
                                    .child(agent.task_count.to_string()),
                            ),
                    )
                    .when_some(run_detail, |inspector, run_detail| {
                        inspector.child(run_detail)
                    })
                    .when(!has_run, |inspector| {
                        inspector.child(
                            div()
                                .pt(px(8.0))
                                .border_t_1()
                                .border_color(theme.border)
                                .text_size(px(10.0))
                                .text_color(theme.muted_foreground)
                                .child("No active run linked to this agent."),
                        )
                    })
                    .into_any_element()
            });

            div()
                .w(px(200.0))
                .flex_shrink_0()
                .h_full()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .py(px(14.0))
                .px(px(10.0))
                .bg(theme.background)
                .border_r_1()
                .border_color(theme.border)
                .id("office-sidebar")
                .overflow_y_scrollbar()
                // Panel header
                .child(
                    div()
                        .w_full()
                        .bg(theme.secondary)
                        .border_1()
                        .border_color(theme.border)
                        .rounded(px(10.0))
                        .overflow_hidden()
                        .child(
                            div()
                                .px(px(14.0))
                                .py(px(10.0))
                                .border_b_1()
                                .border_color(theme.border)
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(theme.muted_foreground)
                                        .child("AGENT WORKSPACE"),
                                ),
                        )
                        .child(agent_list),
                )
                .when_some(agent_inspector, |sidebar, inspector| {
                    sidebar.child(inspector)
                })
                // Stats
                .child(
                    v_flex()
                        .gap(px(6.0))
                        .child(
                            h_flex()
                                .px(px(12.0))
                                .py(px(6.0))
                                .bg(theme.secondary)
                                .border_1()
                                .border_color(theme.border)
                                .rounded(px(8.0))
                                .gap(px(8.0))
                                .items_center()
                                .text_size(px(11.0))
                                .text_color(theme.muted_foreground)
                                .child(
                                    div()
                                        .w(px(6.0))
                                        .h(px(6.0))
                                        .rounded_full()
                                        .bg(Hsla::from(gpui::rgba(0x4ade80ff))),
                                )
                                .child("LIVE")
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(theme.foreground)
                                        .child({
                                            let now = chrono::Local::now();
                                            now.format("%H:%M").to_string()
                                        }),
                                ),
                        )
                        .child(
                            h_flex()
                                .px(px(12.0))
                                .py(px(6.0))
                                .bg(theme.secondary)
                                .border_1()
                                .border_color(theme.border)
                                .rounded(px(8.0))
                                .gap(px(8.0))
                                .text_size(px(11.0))
                                .text_color(theme.muted_foreground)
                                .child("Agents")
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(theme.foreground)
                                        .child(format!("{}", agent_count)),
                                ),
                        ),
                )
        };

        // Canvas area: the main map rendering
        let canvas_element = {
            let state_snapshot = self.office_state.clone();
            let view_for_mouse = view.clone();
            let view_for_bounds = view.clone();
            let quick_chat_overlay = self.render_office_quick_chat_overlay(cx);
            let reduce_text_paint = quick_chat_open;

            div()
                .relative()
                .flex_1()
                .w_full()
                .min_w_0()
                .min_h(px(280.0))
                .id("office-canvas")
                .overflow_hidden()
                .on_mouse_down(MouseButton::Left, {
                    let view_md = view.clone();
                    move |event, _window, cx| {
                        let _ = view_md.update(cx, |this, cx| {
                            // Hit test: find agent under click
                            // We need the canvas bounds, approximate from the event position
                            let pos = event.position;
                            this.office_handle_mouse_down(pos, cx);
                        });
                    }
                })
                .on_mouse_down(MouseButton::Right, {
                    let view_context = view.clone();
                    move |event, window, cx| {
                        cx.stop_propagation();
                        let pos = event.position;
                        let _ = view_context.update(cx, |this, cx| {
                            this.office_open_quick_chat_at(pos, window, cx);
                        });
                    }
                })
                .on_mouse_move({
                    let view_mm = view.clone();
                    move |event, _window, cx| {
                        let _ = view_mm.update(cx, |this, cx| {
                            if this.office_state.dragged_agent_idx.is_some() {
                                this.office_handle_mouse_move(event.position, cx);
                            }
                        });
                    }
                })
                .on_mouse_up(MouseButton::Left, {
                    let view_mu = view_for_mouse;
                    move |_event, _window, cx| {
                        let _ = view_mu.update(cx, |this, cx| {
                            this.office_state.dragged_agent_idx = None;
                            cx.notify();
                        });
                    }
                })
                .child(
                    canvas(
                        move |bounds, _window, cx| {
                            let _ = view_for_bounds.update(cx, |this, _cx| {
                                this.office_canvas_bounds_cache = Some(bounds);
                            });
                        },
                        move |bounds, _, window, cx| {
                            window.with_content_mask(Some(ContentMask { bounds }), |window| {
                                paint_office(
                                    bounds,
                                    &state_snapshot,
                                    reduce_text_paint,
                                    window,
                                    cx,
                                );
                            });
                        },
                    )
                    .size_full(),
                )
                .when_some(quick_chat_overlay, |container, overlay| {
                    container.child(overlay)
                })
        };
        let chat_panel = self.render_office_chat_panel(cx);

        if should_continue_office_animation && !self.office_animation_queued {
            self.office_animation_queued = true;
            cx.spawn({
                let view_anim = view.clone();
                async move |_, cx| {
                    cx.background_executor().timer(office_animation_delay).await;
                    let _ = cx.update(|cx| {
                        let _ = view_anim.update(cx, |this, cx| {
                            this.office_animation_queued = false;
                            cx.notify();
                        });
                    });
                }
            })
            .detach();
        }

        h_flex().size_full().overflow_hidden().child(sidebar).child(
            v_flex()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .size_full()
                .child(canvas_element)
                .child(chat_panel),
        )
    }

    // Store last known canvas bounds for mouse hit testing
    pub(crate) fn office_canvas_bounds(&self) -> Option<Bounds<Pixels>> {
        self.office_canvas_bounds_cache
    }

    fn office_agent_index_at_position(
        &self,
        pos: Point<Pixels>,
        include_static_agents: bool,
    ) -> Option<usize> {
        let bounds = self.office_canvas_bounds()?;
        let layout = OfficeLayout::from_bounds(bounds);
        let pos_x: f32 = pos.x.into();
        let pos_y: f32 = pos.y.into();

        self.office_state
            .agents
            .iter()
            .enumerate()
            .rev()
            .find_map(|(idx, agent)| {
                if !include_static_agents && is_coordinator_identity(&agent.id, &agent.name) {
                    return None;
                }

                let center = layout.agent_center(agent);
                let center_x: f32 = center.x.into();
                let center_y: f32 = center.y.into();
                let dx = (pos_x - center_x).abs();
                let dy = (pos_y - center_y).abs();

                (dx < 32.0 && dy < 42.0).then_some(idx)
            })
    }

    fn office_handle_mouse_down(&mut self, pos: Point<Pixels>, cx: &mut Context<Self>) {
        if let Some(idx) = self.office_agent_index_at_position(pos, false) {
            if let Some(agent) = self.office_state.agents.get(idx) {
                self.office_state.dragged_agent_idx = Some(idx);
                self.selected_office_agent_id = Some(agent.id.clone());
                cx.notify();
            }
        } else if self.office_quick_chat_agent_id.is_some() {
            self.office_quick_chat_agent_id = None;
            cx.notify();
        }
    }

    fn office_open_quick_chat_at(
        &mut self,
        pos: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(idx) = self.office_agent_index_at_position(pos, true) else {
            return;
        };
        let Some(agent) = self.office_state.agents.get(idx).cloned() else {
            return;
        };

        self.selected_office_agent_id = Some(agent.id.clone());
        self.office_quick_chat_agent_id = Some(agent.id.clone());
        self.office_quick_chat_input_state.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
        cx.notify();
    }

    fn render_office_quick_chat_overlay(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let agent_id = self.office_quick_chat_agent_id.as_ref()?;
        let agent = self
            .office_state
            .agents
            .iter()
            .find(|agent| &agent.id == agent_id)?;
        let bounds = self.office_canvas_bounds()?;
        let layout = OfficeLayout::from_bounds(bounds);
        let center = layout.agent_center(agent);
        let theme = cx.theme().clone();

        let bounds_w: f32 = bounds.size.width.into();
        let bounds_h: f32 = bounds.size.height.into();
        let origin_x: f32 = bounds.origin.x.into();
        let origin_y: f32 = bounds.origin.y.into();
        let center_x: f32 = center.x.into();
        let center_y: f32 = center.y.into();

        let panel_w = if bounds_w < 376.0 {
            (bounds_w - 16.0).max(220.0)
        } else {
            360.0
        };
        let panel_h = 158.0;
        let rel_x = center_x - origin_x;
        let rel_y = center_y - origin_y;
        let max_left = (bounds_w - panel_w - 8.0).max(8.0);
        let left = (rel_x - panel_w / 2.0).clamp(8.0, max_left);
        let below_top = rel_y + 36.0;
        let max_top = (bounds_h - panel_h - 8.0).max(8.0);
        let top = if below_top <= max_top {
            below_top
        } else {
            (rel_y - panel_h - 52.0).max(8.0)
        };

        let target_agent_id = agent.id.clone();
        let target_name = agent.name.clone();
        let title = "CURRENT AGENT";
        let helper = "Add message as context for this agent";

        Some(
            v_flex()
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(panel_w))
                .h(px(panel_h))
                .rounded(px(8.0))
                .border_1()
                .border_color(theme.border)
                .bg(theme.background.opacity(0.98))
                .overflow_hidden()
                .p(px(8.0))
                .gap(px(7.0))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .px(px(2.0))
                        .child(
                            div()
                                .text_size(px(11.0))
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(theme.muted_foreground)
                                .child(title),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(theme.foreground)
                                .child(target_name.clone()),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .h(px(76.0))
                        .min_h(px(76.0))
                        .max_h(px(76.0))
                        .rounded(px(7.0))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.background)
                        .overflow_hidden()
                        .px(px(8.0))
                        .py(px(6.0))
                        .child(
                            Input::new(&self.office_quick_chat_input_state)
                                .appearance(false)
                                .w_full()
                                .h_full()
                                .overflow_hidden(),
                        ),
                )
                .child(
                    h_flex()
                        .w_full()
                        .h(px(30.0))
                        .flex_shrink_0()
                        .items_center()
                        .justify_between()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .truncate()
                                .text_size(px(11.0))
                                .text_color(theme.muted_foreground)
                                .child(helper),
                        )
                        .child(
                            Button::new(SharedString::from(format!(
                                "office-quick-chat-send-{}",
                                target_agent_id
                            )))
                            .small()
                            .primary()
                            .label("Send")
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    let text = this
                                        .office_quick_chat_input_state
                                        .read(cx)
                                        .text()
                                        .to_string();
                                    let text = text.trim().to_string();
                                    if text.is_empty() {
                                        return;
                                    }

                                    this.selected_office_agent_id = Some(target_agent_id.clone());
                                    this.office_chat_target_agent_id =
                                        Some(target_agent_id.clone());
                                    this.office_quick_chat_agent_id = None;
                                    this.office_quick_chat_input_state.update(cx, |state, cx| {
                                        state.set_value("", window, cx);
                                    });
                                    this.chat_input_state.update(cx, |state, cx| {
                                        state.set_value(&text, window, cx);
                                    });
                                    this.handle_send_chat(window, cx);
                                },
                            )),
                        ),
                )
                .into_any_element(),
        )
    }

    fn office_handle_mouse_move(&mut self, pos: Point<Pixels>, cx: &mut Context<Self>) {
        if let Some(idx) = self.office_state.dragged_agent_idx {
            if let Some(bounds) = self.office_canvas_bounds() {
                let layout = OfficeLayout::from_bounds(bounds);
                if let Some(agent) = self.office_state.agents.get_mut(idx) {
                    let (x, y) = layout.agent_target_from_window_pos(pos);
                    let (x, y) = clamp_agent_position(x, y, (agent.tx, agent.ty));
                    agent.tx = x;
                    agent.ty = y;
                    cx.notify();
                }
            }
        }
    }
}
