use gpui::{div, App, IntoElement, ParentElement, RenderOnce, Styled, Window};

pub struct CommandItem {
    pub label: String,
    pub description: String,
}

pub struct CommandPalette {
    pub items: Vec<CommandItem>,
    pub query: String,
}

impl CommandPalette {
    pub fn new(items: Vec<CommandItem>) -> Self {
        Self {
            items,
            query: String::new(),
        }
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
    }
}

impl RenderOnce for CommandPalette {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut container = div().flex().flex_col().gap_2();

        let filtered_items = self.items.into_iter().filter(|i| {
            self.query.is_empty() || i.label.to_lowercase().contains(&self.query.to_lowercase())
        });

        for item in filtered_items {
            container = container.child(
                div()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .child(div().child(item.label))
                    .child(div().child(item.description)),
            );
        }

        container
    }
}
