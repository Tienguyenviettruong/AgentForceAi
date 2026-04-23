# GPUI Component (gpui-component) — Technical Guide (v0.5.1)
Date: 2026-04-06  
Target version: **gpui-component v0.5.1** / **gpui v0.2.2**  

## 1. What is gpui-component?
**gpui-component** is a Rust UI component library built on top of **GPUI** for building cross‑platform desktop applications. It provides a large set of reusable UI primitives (buttons, inputs, lists, tables, markdown, charts, a code editor, etc.), a theming system, and a component gallery (“story”) for development and QA.  

Key capabilities highlighted by the upstream project:
- 60+ desktop UI components
- Theme + ThemeColor (multi-theme, variable-driven)
- Dock layout, tiles, virtualization for lists/tables
- Markdown + basic HTML rendering
- Built-in charts
- High performance code editor with Tree-sitter highlighting + LSP support  

## 2. Installation & versioning
Add dependencies in your `Cargo.toml`:
```toml
[dependencies]
gpui = "0.2.2"
gpui-component = "0.5.1"
```

### Compatibility notes
- The library targets desktop apps (GPUI).  
- Some advanced capabilities like **WebView** are experimental and OS-limited (see §10).  
- The docs site “Quick Example” currently shows `gpui = "0.5.1"`, but **the published `gpui-component v0.5.1` crate depends on `gpui ^0.2.2`** (Cargo will enforce this). Prefer the `gpui` version implied by `gpui-component`’s crate dependency graph.

## 3. Project layout (upstream repository structure)
The upstream repo is organized into multiple crates:
- `crates/ui`: primary component library (most components live here)
- `crates/story`: desktop component gallery / demos
- `crates/story-web`: WASM-based story gallery deployed to GitHub Pages
- `crates/webview`: WebView integration (Wry-based; experimental)
- `crates/macros`: helper macros (e.g., icon generation)
- `crates/assets`: assets bundle (icons, etc.)

This structure matters because:
- You typically depend on **`gpui-component`** and call `gpui_component::init(cx)` once at app startup.
- You reference `crates/story` and `examples/` as working examples for patterns and API usage.
- You may optionally embed `crates/webview` if you need web content inside a GPUI window.

## 4. Quickstart: minimal window + Root + Button
### 4.1 The critical `init(cx)` call
The upstream README emphasizes you must initialize gpui-component before using its features:
```rust
fn main() {
    let app = Application::new();

    app.run(move |cx| {
        // Must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| HelloWorld);
                // This first level on the window should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
```

### 4.2 A simple Render component
```rust
use gpui::*;
use gpui_component::{button::*, *};

pub struct HelloWorld;

impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("ok")
                    .primary()
                    .label("Let's Go!")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}
```

## 5. Theming & styling model (practical mental model)
gpui-component is designed around:
- A centralized **Theme / ThemeColor** model (light/dark and variants)
- Fluent builders (e.g., `div().v_flex().gap_2()...`)
- Consistent sizing tokens (`xs`, `sm`, `md`, `lg`), spacing, and component defaults

### 5.1 How to approach theming in your app
Recommended approach:
1. Pick a base theme (dark/light).
2. Use the component defaults first.
3. Only then introduce customization at:
   - app-level theme configuration
   - component-level overrides (only where needed)

### 5.2 Layout primitives (GPUI fluent builder)
Common patterns:
- `h_flex()` / `v_flex()` for layout direction
- `gap_*()` to enforce consistent spacing
- `size_full()` for fill behavior
- `items_center()` / `justify_center()` for alignment

## 6. Component categories & usage patterns
Below is a practical taxonomy to help you navigate gpui-component.

### 6.1 “Core primitives”
Used everywhere:
- Buttons (primary/secondary/ghost, disabled, loading)
- Inputs (text input, textarea, validation states)
- Selects / dropdowns / menus
- Tabs
- Tooltips / popovers
- Badges / chips / tags
- Dialogs / overlays

Usage pattern:
- Prefer stateless components and pass state via GPUI entities/context.
- Use `on_click`, `on_change`, etc. closures to bind interactions.

### 6.2 Navigation + layout systems
gpui-component supports advanced desktop layout patterns:
- **Dock layout**: split panes, resizable panels
- **Tiles** / freeform layout (useful for dashboards / multi-view workspaces)
- Sidebars / toolbars / top bars (compose using layout primitives)

#### 6.2.1 TitleBar (`gpui_component::TitleBar`)

A native-feeling window title bar component that integrates with the GPUI window chrome. It provides a horizontal bar where you place application menus on the left and custom controls (buttons, badges, dropdowns) on the right.

**Import & basic usage:**
```rust
use gpui_component::TitleBar;
```

**Construction:**
```rust
TitleBar::new()
    // Left side — typically the app menu bar
    .child(div().flex().items_center().child(app_menu_bar.clone()))
    // Right side — custom controls
    .child(
        div()
            .flex()
            .items_center()
            .justify_end()
            .px_2()
            .gap_2()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(Button::new("settings").icon(IconName::Settings2).small().ghost())
            .child(
                Badge::new().count(notifications_count).max(99).child(
                    Button::new("bell").small().ghost().compact().icon(IconName::Bell),
                ),
            ),
    )
```

**Key characteristics:**
- Uses a fluent builder pattern: `TitleBar::new()` then chain `.child(...)` calls.
- Children are laid out horizontally; left-side children are placed first, right-side children flow to the end.
- Supports `on_mouse_down` with `cx.stop_propagation()` to prevent drag-to-move when clicking controls.
- Commonly combined with `AppMenuBar` on the left and `Button`/`Badge`/`DropdownMenu` on the right.
- Integrates with `WindowExt` for platform-native title bar behavior.
- The title bar respects the active theme (background, text color, border).

**Typical integration pattern (wrapping in a custom struct):**
```rust
use std::rc::Rc;
use gpui::*;
use gpui_component::{ActiveTheme as _, IconName, TitleBar, WindowExt as _,
    badge::Badge, button::{Button, ButtonVariants as _}, label::Label,
    menu::{AppMenuBar, DropdownMenu as _}};

pub struct AppTitleBar {
    app_menu_bar: Entity<AppMenuBar>,
    child: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
    _subscriptions: Vec<Subscription>,
}

impl AppTitleBar {
    pub fn new(title: impl Into<SharedString>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let app_menu_bar = app_menus::init(title, cx);
        Self {
            app_menu_bar,
            child: Rc::new(|_, _| div().into_any_element()),
            _subscriptions: vec![],
        }
    }

    /// Allow callers to inject custom right-side controls.
    pub fn child<F, E>(mut self, f: F) -> Self
    where E: IntoElement, F: Fn(&mut Window, &mut App) -> E + 'static {
        self.child = Rc::new(move |window, cx| f(window, cx).into_any_element());
        self
    }
}

impl Render for AppTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        TitleBar::new()
            .child(div().flex().items_center().child(self.app_menu_bar.clone()))
            .child(
                div().flex().items_center().justify_end().px_2().gap_2()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child((self.child.clone())(window, cx))
            )
    }
}
```

#### 6.2.2 AppMenuBar (`gpui_component::menu::AppMenuBar`)

A native-style application menu bar (File / Edit / Window / Help) that renders inside the `TitleBar`. Menus are built using the `Menu` and `MenuItem` types and support actions, submenus, separators, and checked items.

**Import & basic usage:**
```rust
use gpui_component::menu::AppMenuBar;
```

**Initialization:**
```rust
let app_menu_bar = AppMenuBar::new(cx);
```

**Building menus:**
```rust
use gpui::{Menu, MenuItem};

let menus = vec![
    Menu {
        name: "MyApp".into(),
        items: vec![
            MenuItem::action("About", About),
            MenuItem::Separator,
            MenuItem::action("Open...", Open),
            MenuItem::Separator,
            MenuItem::Submenu(Menu {
                name: "Appearance".into(),
                items: vec![
                    MenuItem::action("Light", SwitchThemeMode(ThemeMode::Light))
                        .checked(!cx.theme().mode.is_dark()),
                    MenuItem::action("Dark", SwitchThemeMode(ThemeMode::Dark))
                        .checked(cx.theme().mode.is_dark()),
                ],
                disabled: false,
            }),
            MenuItem::Separator,
            MenuItem::action("Quit", Quit),
        ],
        disabled: false,
    },
    Menu {
        name: "Edit".into(),
        items: vec![
            MenuItem::action("Undo", gpui_component::input::Undo),
            MenuItem::action("Redo", gpui_component::input::Redo),
            MenuItem::separator(),
            MenuItem::action("Cut", gpui_component::input::Cut),
            MenuItem::action("Copy", gpui_component::input::Copy),
            MenuItem::action("Paste", gpui_component::input::Paste),
            MenuItem::separator(),
            MenuItem::action("Find", gpui_component::input::Search),
            MenuItem::separator(),
            MenuItem::action("Select All", gpui_component::input::SelectAll),
        ],
        disabled: false,
    },
];
```

**Registering menus with the system:**
```rust
// Set platform-native menus (e.g., macOS menu bar)
cx.set_menus(menus.clone());

// Also register as in-window menu bar via GlobalState
let owned: Vec<_> = menus.into_iter().map(|m| m.owned()).collect();
GlobalState::global_mut(cx).set_app_menus(owned);

// Reload the in-window menu bar to pick up changes
app_menu_bar.update(cx, |menu_bar, cx| menu_bar.reload(cx));
```

**Key characteristics:**
- `Menu` struct: has `name` (SharedString), `items` (Vec<MenuItem>), and `disabled` flag.
- `MenuItem` variants:
  - `MenuItem::action(label, action)` — triggers a GPUI action when clicked.
  - `MenuItem::Separator` — visual divider line.
  - `MenuItem::Submenu(Menu { .. })` — nested submenu.
  - `.checked(bool)` — shows a checkmark next to the item (e.g., for toggle states like theme selection).
  - `.disabled(bool)` — grays out the item.
- Built-in input actions are provided: `Undo`, `Redo`, `Cut`, `Copy`, `Paste`, `Delete`, `SelectAll`, `Search`, `DeleteToPreviousWordStart`, `DeleteToNextWordEnd`.
- Dynamic updates: call `menu_bar.reload(cx)` after modifying menus. Use `cx.observe_global::<Theme>(...)` to reactively update checked states (e.g., when theme changes).
- Supports internationalization: rebuild menus on locale change to update labels.
- The `DropdownMenu` component (used on `Button`) shares the same menu item types, providing consistency between the menu bar and context menus.

### 6.3 Lists & tables (virtualization)
For large datasets:
- Prefer built-in list/table components that support virtualization.
- Keep row render functions lightweight.

Typical use cases:
- logs / audit trails
- file trees
- session lists
- metrics tables

### 6.4 Markdown & content rendering
gpui-component provides markdown rendering, and supports “markdown mix HTML” to some degree (per upstream comparison table).

Practical usage:
- Chat transcripts
- Documentation preview
- Release notes / audit summaries

### 6.5 Charts
Built-in charts are useful for:
- token usage dashboards
- performance/latency views
- cost trend charts

Guidance:
- For dense dashboards, keep legends simple and use consistent color palettes.
- Prefer a small number of chart types to reduce UI complexity.

### 6.6 Code editor (Tree-sitter + LSP)
The upstream project advertises a high-performance editor supporting up to ~200K lines, with:
- LSP: diagnostics, completion, hover
- Tree-sitter: syntax highlighting (also used for markdown)

Practical usage:
- embedded code viewer/editor in developer tools (like AgentForge)
- JSON/YAML editors for configuration
- prompt/skill editors

### 6.7 Docs site component catalog (official)
The official docs site contains a component catalog and per-component pages with:
- **Import snippet** (the canonical `use gpui_component::...` path)
- **Usage examples**
- Often an **API reference** section linking to `docs.rs`

Start here (catalog):  
https://longbridge.github.io/gpui-component/docs/components/index

#### Catalog (from docs site)
**Basic Components**
- Accordion — https://longbridge.github.io/gpui-component/docs/components/accordion
- Alert — https://longbridge.github.io/gpui-component/docs/components/alert
- Avatar — https://longbridge.github.io/gpui-component/docs/components/avatar
- Badge — https://longbridge.github.io/gpui-component/docs/components/badge
- Button — https://longbridge.github.io/gpui-component/docs/components/button
- Checkbox — https://longbridge.github.io/gpui-component/docs/components/checkbox
- Collapsible — https://longbridge.github.io/gpui-component/docs/components/collapsible
- DropdownButton — https://longbridge.github.io/gpui-component/docs/components/dropdown_button
- Icon — https://longbridge.github.io/gpui-component/docs/components/icon
- Image — https://longbridge.github.io/gpui-component/docs/components/image
- Kbd — https://longbridge.github.io/gpui-component/docs/components/kbd
- Label — https://longbridge.github.io/gpui-component/docs/components/label
- Pagination — https://longbridge.github.io/gpui-component/docs/components/pagination
- Progress — https://longbridge.github.io/gpui-component/docs/components/progress
- Radio — https://longbridge.github.io/gpui-component/docs/components/radio
- Rating — https://longbridge.github.io/gpui-component/docs/components/rating
- Skeleton — https://longbridge.github.io/gpui-component/docs/components/skeleton
- Slider — https://longbridge.github.io/gpui-component/docs/components/slider
- Spinner — https://longbridge.github.io/gpui-component/docs/components/spinner
- Stepper — https://longbridge.github.io/gpui-component/docs/components/stepper
- Switch — https://longbridge.github.io/gpui-component/docs/components/switch
- Tag — https://longbridge.github.io/gpui-component/docs/components/tag
- Toggle — https://longbridge.github.io/gpui-component/docs/components/toggle
- Tooltip — https://longbridge.github.io/gpui-component/docs/components/tooltip

**Form Components**
- Input — https://longbridge.github.io/gpui-component/docs/components/input
- Select — https://longbridge.github.io/gpui-component/docs/components/select
- NumberInput — https://longbridge.github.io/gpui-component/docs/components/number-input
- DatePicker — https://longbridge.github.io/gpui-component/docs/components/date-picker
- OtpInput — https://longbridge.github.io/gpui-component/docs/components/otp-input
- ColorPicker — https://longbridge.github.io/gpui-component/docs/components/color-picker
- Editor — https://longbridge.github.io/gpui-component/docs/components/editor
- Form — https://longbridge.github.io/gpui-component/docs/components/form

**Layout Components**
- DescriptionList — https://longbridge.github.io/gpui-component/docs/components/description-list
- GroupBox — https://longbridge.github.io/gpui-component/docs/components/group-box
- Dialog — https://longbridge.github.io/gpui-component/docs/components/dialog
- Notification — https://longbridge.github.io/gpui-component/docs/components/notification
- Popover — https://longbridge.github.io/gpui-component/docs/components/popover
- Resizable — https://longbridge.github.io/gpui-component/docs/components/resizable
- Scrollable — https://longbridge.github.io/gpui-component/docs/components/scrollable
- Sheet — https://longbridge.github.io/gpui-component/docs/components/sheet
- Sidebar — https://longbridge.github.io/gpui-component/docs/components/sidebar

**Advanced Components**
- Calendar — https://longbridge.github.io/gpui-component/docs/components/calendar
- Chart — https://longbridge.github.io/gpui-component/docs/components/chart
- List — https://longbridge.github.io/gpui-component/docs/components/list
- Menu — https://longbridge.github.io/gpui-component/docs/components/menu
- Settings — https://longbridge.github.io/gpui-component/docs/components/settings
- DataTable — https://longbridge.github.io/gpui-component/docs/components/data-table
- Tabs — https://longbridge.github.io/gpui-component/docs/components/tabs
- Tree — https://longbridge.github.io/gpui-component/docs/components/tree
- VirtualList — https://longbridge.github.io/gpui-component/docs/components/virtual-list

#### Examples of “Import” + API reference patterns (from the docs pages)
- **Accordion**
  - Import: `use gpui_component::accordion::Accordion;`
  - API: https://docs.rs/gpui-component/latest/gpui_component/accordion/struct.Accordion.html
- **AlertDialog**
  - Import: `use gpui_component::dialog::{AlertDialog, DialogAction, DialogClose};` + `use gpui_component::WindowExt;`
  - Docs page: https://longbridge.github.io/gpui-component/docs/components/alert-dialog
- **Editor** (multi-line input / code editor)
  - Import: `use gpui_component::input::{InputState, Input};`
  - Docs page: https://longbridge.github.io/gpui-component/docs/components/editor
- **Select**
  - Import: `use gpui_component::select::{ Select, SelectState, ... };`
  - Docs page: https://longbridge.github.io/gpui-component/docs/components/select

## 7. Icons and asset pipeline
gpui-component provides an `Icon` element but does not ship icon SVGs by default. The recommended approach is to use your own icon set (e.g., Lucide) and name SVG files according to `IconName`.

### 7.1 IconName + icon generation
The library uses a macro to generate an `IconName` enum from an assets folder (see `icon_named!` usage in `icon.rs`), and provides `Icon` rendering via GPUI’s `svg()` pipeline.

Practical steps:
1. Add SVG icons to your project’s assets (or a bundle).
2. Name them to match `IconName` variants (or implement your own `IconNamed` trait).
3. Render icons as `Icon::new(...)` or `IconName::view(...)`.

## 8. Examples and “Story” gallery (how to learn the library)
### 8.1 Examples folder
The upstream `examples/` folder contains focused examples designed to demonstrate “one feature per example”.

Recommended workflow:
- Start with `examples/hello_world`.
- Look at examples for input, dialog overlays, focus traps, webview, etc.

### 8.2 Desktop story gallery
The repo provides a story/gallery app for exploring components:
```bash
cargo run
```
This is a fast way to inspect component behavior and styling.

### 8.3 Web story gallery (WASM)
There is also a WASM-based gallery (`crates/story-web`) with a live demo and local dev server:
```bash
cd crates/story-web
make install
make dev
```

## 9. Docs site (VitePress)
The repository contains a docs site powered by VitePress (in `/docs`).
Local dev (per upstream):
```bash
bun install
bun run dev
```

Practical navigation:
- Docs home: https://longbridge.github.io/gpui-component/docs/
- Component catalog: https://longbridge.github.io/gpui-component/docs/components/index
- Individual components are under: `.../docs/components/<slug>`

## 10. WebView support (experimental)
gpui-component includes a Wry-based WebView crate. Key constraints called out by upstream:
- WebView renders on top of the GPUI window (covers any GPUI elements behind it).
- Only supports **macOS and Windows** currently.
- Recommended: use WebView in a separate window or popup layer.

This is relevant when you need to embed a web-based editor (e.g., Milkdown/MDXEditor) inside a GPUI app.

## 11. Common integration patterns (recommended for real apps)
### 11.1 “App shell” with dock layout
Typical structure:
- Root window
- Dock layout: left sidebar, center content, right inspector/chat, bottom status bar
- Route view state via entities (session/team selection, open documents, etc.)

### 11.2 “Viewer + editor” separation
For performance and predictability:
- Use markdown renderers and code viewers for read-only surfaces.
- Use dedicated editor components (or WebView) only where editing is needed.

### 11.3 Performance tips
- Virtualize large lists/tables.
- Avoid heavy allocations in render loops.
- Prefer incremental UI updates (only update entities that changed).

## 12. Reference links (primary sources)
### Upstream repositories & docs
- gpui-component repo: https://github.com/longbridge/gpui-component
- Releases (v0.5.1): https://github.com/longbridge/gpui-component/releases/tag/v0.5.1
- Crates directory: https://github.com/longbridge/gpui-component/tree/main/crates
- `crates/ui` (main component library): https://github.com/longbridge/gpui-component/tree/main/crates/ui
- Examples directory: https://github.com/longbridge/gpui-component/tree/main/examples
- Docs directory (VitePress): https://github.com/longbridge/gpui-component/tree/main/docs
- Docs site home: https://longbridge.github.io/gpui-component/docs/
- Docs site component catalog: https://longbridge.github.io/gpui-component/docs/components/index
- Crates.io dependency graph (shows `gpui ^0.2.2` for `gpui-component v0.5.1`): https://crates.io/crates/gpui-component/0.5.1/dependencies

### Story/gallery
- Live gallery (GitHub Pages): https://longbridge.github.io/gpui-component/gallery/
- `crates/story`: https://github.com/longbridge/gpui-component/tree/main/crates/story
- `crates/story-web` README: https://github.com/longbridge/gpui-component/tree/main/crates/story-web#readme

### Icons
- `IconName` / Icon implementation (`icon.rs`): https://github.com/longbridge/gpui-component/blob/main/crates/ui/src/icon.rs
- Lucide icons (suggested icon set): https://lucide.dev/

### WebView
- `crates/webview` README: https://github.com/longbridge/gpui-component/tree/main/crates/webview#readme
- Wry (underlying WebView project): https://github.com/tauri-apps/wry

### GPUI
- GPUI website: https://gpui.rs/
