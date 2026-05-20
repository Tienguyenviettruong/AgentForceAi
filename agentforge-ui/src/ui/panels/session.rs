use crate::application::iflow_engine::engine::Workflow;
use gpui::EventEmitter;
use gpui::{
    div, App, Context, Focusable, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    Window,
};
use gpui_component::button::ButtonVariants;
use gpui_component::dock::PanelEvent;
use gpui_component::dock::{Panel, TitleStyle};
use gpui_component::scroll::ScrollableElement;
use gpui_component::StyledExt;
use gpui_component::{button::Button, h_flex, theme::ActiveTheme, v_flex};

#[derive(Clone, Debug, PartialEq)]
pub enum SkillStatus {
    Learning,
    Active,
    Paused,
}

#[derive(Clone, Debug)]
pub struct LearnedSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: SkillStatus,
    pub steps: Vec<String>,
    pub execution_count: usize,
}

pub struct SessionPanel {
    focus_handle: gpui::FocusHandle,
    skills: Vec<LearnedSkill>,
    selected_skill_id: Option<String>,
}

impl SessionPanel {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        let db = crate::AppState::global(cx).db.clone();
        let mut skills = Vec::new();

        if let Ok(workflows) = db.list_workflows() {
            for wf in workflows {
                let mut steps = Vec::new();
                if let Ok(workflow_def) = serde_json::from_str::<Workflow>(&wf.definition) {
                    for (_, node) in workflow_def.nodes {
                        steps.push(node.name);
                    }
                } else {
                    steps.push("Failed to parse workflow definition".to_string());
                }

                skills.push(LearnedSkill {
                    id: wf.id.clone(),
                    name: wf.name,
                    description: "Learned from MCP tool actions.".to_string(),
                    status: SkillStatus::Active,
                    steps,
                    execution_count: 0,
                });
            }
        }

        let selected_skill_id = skills.first().map(|s| s.id.clone());

        Self {
            focus_handle: cx.focus_handle(),
            skills,
            selected_skill_id,
        }
    }
}

impl Panel for SessionPanel {
    fn panel_name(&self) -> &'static str {
        "Skills & Behaviors"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.panel_name()
    }

    fn title_style(&self, _cx: &App) -> Option<TitleStyle> {
        None
    }
}

impl Focusable for SessionPanel {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SessionPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let mut sidebar = v_flex()
            .w_1_3()
            .h_full()
            .border_r_1()
            .border_color(theme.border)
            .bg(theme.background);

        let mut list_container = v_flex().flex_1().overflow_y_scrollbar();

        for skill in &self.skills {
            let is_selected = self.selected_skill_id == Some(skill.id.clone());
            let bg_color = if is_selected {
                theme.secondary
            } else {
                gpui::transparent_black()
            };

            let (status_text, status_color) = match skill.status {
                SkillStatus::Active => ("Active", gpui::green()),
                SkillStatus::Paused => ("Paused", gpui::yellow()),
                SkillStatus::Learning => ("Learning", gpui::blue()),
            };

            let skill_id = skill.id.clone();
            let item = v_flex()
                .p_3()
                .border_b_1()
                .border_color(theme.border)
                .bg(bg_color)
                .cursor_pointer()
                .hover(|s| s.bg(theme.secondary))
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        this.selected_skill_id = Some(skill_id.clone());
                        cx.notify();
                    }),
                )
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .mb_1()
                        .child(
                            div()
                                .text_sm()
                                .font_bold()
                                .text_color(theme.foreground)
                                .child(skill.name.clone()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .font_bold()
                                .text_color(status_color)
                                .child(status_text),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(format!("{} executions", skill.execution_count)),
                );

            list_container = list_container.child(item);
        }
        sidebar = sidebar.child(list_container);

        let mut main_content = v_flex().flex_1().h_full().bg(theme.background).p_6();

        if let Some(selected_id) = &self.selected_skill_id {
            if let Some(skill) = self.skills.iter_mut().find(|s| s.id == *selected_id) {
                let header = h_flex()
                    .justify_between()
                    .items_start()
                    .mb_6()
                    .child(
                        v_flex()
                            .child(
                                div()
                                    .text_xl()
                                    .font_bold()
                                    .text_color(theme.foreground)
                                    .child(skill.name.clone()),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .mt_2()
                                    .child(skill.description.clone()),
                            ),
                    )
                    .child(h_flex().gap_2().items_center().child({
                        let mut btn = Button::new("toggle-status").label(
                            if skill.status == SkillStatus::Active {
                                "Pause Skill"
                            } else {
                                "Enable Auto-Run"
                            },
                        );
                        if skill.status == SkillStatus::Active {
                            btn = btn; // just default style
                        } else {
                            btn = btn.primary();
                        }
                        let sid = selected_id.clone();
                        btn.on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                if let Some(s) = this.skills.iter_mut().find(|s| s.id == sid) {
                                    s.status = if s.status == SkillStatus::Active {
                                        SkillStatus::Paused
                                    } else {
                                        SkillStatus::Active
                                    };
                                    cx.notify();
                                }
                            }),
                        )
                    }));

                let mut steps_list = v_flex().gap_3().mt_4();
                for (i, step) in skill.steps.iter().enumerate() {
                    steps_list = steps_list.child(
                        h_flex()
                            .items_start()
                            .gap_3()
                            .child(
                                div()
                                    .w(gpui::px(24.))
                                    .h(gpui::px(24.))
                                    .rounded_full()
                                    .bg(theme.secondary)
                                    .flex()
                                    .justify_center()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_bold()
                                            .text_color(theme.foreground)
                                            .child(format!("{}", i + 1)),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .p_3()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(theme.border)
                                    .bg(theme.background)
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.foreground)
                                            .child(step.clone()),
                                    ),
                            ),
                    );
                }

                main_content = main_content
                    .child(header)
                    .child(div().h(gpui::px(1.)).w_full().bg(theme.border).my_4())
                    .child(
                        div()
                            .text_sm()
                            .font_bold()
                            .text_color(theme.foreground)
                            .mb_4()
                            .child("Workflow Steps (Extracted from behavior)"),
                    )
                    .child(div().flex_1().overflow_y_scrollbar().child(steps_list));
            }
        } else {
            main_content = main_content.child(
                div()
                    .size_full()
                    .flex()
                    .justify_center()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Select a learned skill to view details"),
                    ),
            );
        }

        h_flex().size_full().child(sidebar).child(main_content)
    }
}

impl EventEmitter<PanelEvent> for SessionPanel {}
