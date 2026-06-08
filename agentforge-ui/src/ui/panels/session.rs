use crate::application::iflow_engine::engine::WorkflowEngine;
use gpui::EventEmitter;
use gpui::{
    div, App, Context, Focusable, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    Window,
};
use gpui_component::dock::PanelEvent;
use gpui_component::dock::{Panel, TitleStyle};
use gpui_component::scroll::ScrollableElement;
use gpui_component::StyledExt;
use gpui_component::{h_flex, switch::Switch, theme::ActiveTheme, v_flex};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub enum SkillStatus {
    Learning,
    Active,
    Paused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionTab {
    AgentSkills,
    LearnedAutomations,
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
    active_tab: SessionTab,
    builtin_skills: Vec<crate::application::skills::SkillMetadata>,
    enabled_skill_ids: HashSet<String>,
    skills: Vec<LearnedSkill>,
    selected_builtin_skill_id: Option<String>,
    selected_skill_id: Option<String>,
}

impl SessionPanel {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        let db = crate::AppState::global(cx).db.clone();
        let builtin_skills = crate::application::skills::builtin_skill_catalog();
        let enabled_skill_ids = db
            .list_capability_selections("default", "")
            .unwrap_or_default()
            .into_iter()
            .filter(|selection| selection.enabled && selection.capability_kind == "skill")
            .map(|selection| selection.capability_id)
            .collect();
        let mut skills = Vec::new();

        if let Ok(workflows) = db.list_workflows() {
            for wf in workflows {
                if wf.origin_kind != "learned" {
                    continue;
                }
                let mut steps = Vec::new();
                if let Ok(workflow_def) = WorkflowEngine::parse_validated_draft(&wf.definition) {
                    for (_, node) in workflow_def.nodes {
                        steps.push(node.name);
                    }
                } else {
                    steps.push("Failed to parse workflow definition".to_string());
                }

                skills.push(LearnedSkill {
                    id: wf.id.clone(),
                    name: wf.name,
                    description: "Learned workflow from recorded MCP actions. Review is required before it can be linked to an execution run.".to_string(),
                    status: match wf.activation_status.as_str() {
                        "active" => SkillStatus::Active,
                        "paused" => SkillStatus::Paused,
                        _ => SkillStatus::Learning,
                    },
                    steps,
                    execution_count: 0,
                });
            }
        }

        let selected_builtin_skill_id = builtin_skills.first().map(|skill| skill.id.clone());
        let selected_skill_id = if selected_builtin_skill_id.is_none() {
            skills.first().map(|s| s.id.clone())
        } else {
            None
        };

        Self {
            focus_handle: cx.focus_handle(),
            active_tab: SessionTab::AgentSkills,
            builtin_skills,
            enabled_skill_ids,
            skills,
            selected_builtin_skill_id,
            selected_skill_id,
        }
    }

    fn set_builtin_skill_enabled(&mut self, skill_id: &str, enabled: bool, cx: &mut Context<Self>) {
        let now = chrono::Utc::now().to_rfc3339();
        let state = crate::AppState::global(cx);
        let _ =
            state
                .db
                .upsert_capability_selection(&crate::core::models::CapabilitySelectionRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    scope_kind: "default".to_string(),
                    scope_id: String::new(),
                    capability_kind: "skill".to_string(),
                    capability_id: skill_id.to_string(),
                    enabled,
                    selected_by: Some(state.current_actor_id.clone()),
                    created_at: now.clone(),
                    updated_at: now,
                });
        if enabled {
            self.enabled_skill_ids.insert(skill_id.to_string());
        } else {
            self.enabled_skill_ids.remove(skill_id);
        }
        cx.notify();
    }
}

impl Panel for SessionPanel {
    fn panel_name(&self) -> &'static str {
        "Learned Automations"
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

        let sidebar = v_flex()
            .w_1_3()
            .h_full()
            .min_h_0()
            .border_r_1()
            .border_color(theme.border)
            .bg(theme.background);

        let agent_tab_active = self.active_tab == SessionTab::AgentSkills;
        let automation_tab_active = self.active_tab == SessionTab::LearnedAutomations;
        let tab_bar = h_flex()
            .gap(gpui::px(6.))
            .p_2()
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex_1()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .border_1()
                    .border_color(if agent_tab_active {
                        theme.accent
                    } else {
                        theme.border
                    })
                    .bg(if agent_tab_active {
                        theme.secondary
                    } else {
                        theme.background
                    })
                    .text_sm()
                    .text_color(if agent_tab_active {
                        theme.foreground
                    } else {
                        theme.muted_foreground
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.secondary))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.active_tab = SessionTab::AgentSkills;
                            this.selected_skill_id = None;
                            if this.selected_builtin_skill_id.is_none() {
                                this.selected_builtin_skill_id =
                                    this.builtin_skills.first().map(|skill| skill.id.clone());
                            }
                            cx.notify();
                        }),
                    )
                    .child(format!("Agent Skills ({})", self.builtin_skills.len())),
            )
            .child(
                div()
                    .flex_1()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .border_1()
                    .border_color(if automation_tab_active {
                        theme.accent
                    } else {
                        theme.border
                    })
                    .bg(if automation_tab_active {
                        theme.secondary
                    } else {
                        theme.background
                    })
                    .text_sm()
                    .text_color(if automation_tab_active {
                        theme.foreground
                    } else {
                        theme.muted_foreground
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.secondary))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.active_tab = SessionTab::LearnedAutomations;
                            this.selected_builtin_skill_id = None;
                            if this.selected_skill_id.is_none() {
                                this.selected_skill_id =
                                    this.skills.first().map(|skill| skill.id.clone());
                            }
                            cx.notify();
                        }),
                    )
                    .child(format!("Automations ({})", self.skills.len())),
            );

        let mut list_container = v_flex().flex_1().min_h_0().overflow_y_scrollbar().p_3();

        if self.active_tab == SessionTab::AgentSkills {
            for skill in self.builtin_skills.clone() {
                let enabled = self.enabled_skill_ids.contains(&skill.id);
                let skill_id = skill.id.clone();
                let skill_name = skill.name.clone();
                let skill_category = skill.category.clone();
                let select_skill_id = skill_id.clone();
                let toggle_skill_id = skill_id.clone();
                let view = cx.entity().clone();
                let is_selected =
                    self.selected_builtin_skill_id.as_deref() == Some(skill.id.as_str());
                let bg_color = if is_selected {
                    theme.secondary
                } else {
                    theme.background
                };
                list_container = list_container.child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .p_3()
                        .mb(gpui::px(6.))
                        .rounded_md()
                        .border_1()
                        .border_color(if is_selected {
                            theme.accent
                        } else {
                            theme.border
                        })
                        .bg(bg_color)
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.secondary))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.selected_builtin_skill_id = Some(select_skill_id.clone());
                                this.selected_skill_id = None;
                                cx.notify();
                            }),
                        )
                        .child(
                            v_flex()
                                .gap(gpui::px(2.))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.foreground)
                                        .child(skill_name),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(skill_category),
                                ),
                        )
                        .child(
                            Switch::new(gpui::SharedString::from(format!(
                                "skill-toggle-{}",
                                skill_id
                            )))
                            .checked(enabled)
                            .tooltip(if enabled {
                                "Disable skill"
                            } else {
                                "Enable skill"
                            })
                            .on_click({
                                let toggle_skill_id = toggle_skill_id.clone();
                                let view = view.clone();
                                move |checked: &bool, _window, cx| {
                                    let _ = view.update(cx, |this, cx| {
                                        this.set_builtin_skill_enabled(
                                            &toggle_skill_id,
                                            *checked,
                                            cx,
                                        );
                                    });
                                }
                            }),
                        ),
                );
            }
        } else if self.skills.is_empty() {
            list_container = list_container.child(
                div()
                    .p_4()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("No learned automations yet."),
            );
        } else {
            for skill in &self.skills {
                let is_selected = self.selected_skill_id == Some(skill.id.clone());
                let bg_color = if is_selected {
                    theme.secondary
                } else {
                    theme.background
                };

                let (status_text, status_color) = match skill.status {
                    SkillStatus::Active => ("Active", gpui::green()),
                    SkillStatus::Paused => ("Paused", gpui::yellow()),
                    SkillStatus::Learning => ("Learning", gpui::blue()),
                };

                let skill_id = skill.id.clone();
                let item = v_flex()
                    .p_3()
                    .mb(gpui::px(6.))
                    .rounded_md()
                    .border_1()
                    .border_color(if is_selected {
                        theme.accent
                    } else {
                        theme.border
                    })
                    .bg(bg_color)
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.secondary))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.selected_builtin_skill_id = None;
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
        }

        let sidebar = sidebar
            .child(
                div()
                    .p_3()
                    .text_sm()
                    .font_bold()
                    .text_color(theme.muted_foreground)
                    .child("Agent Skills"),
            )
            .child(tab_bar)
            .child(list_container);

        let mut main_content = v_flex()
            .flex_1()
            .h_full()
            .min_h_0()
            .bg(theme.background)
            .p_6();

        if let Some(selected_builtin_id) = self.selected_builtin_skill_id.clone() {
            if let Some(skill) = self
                .builtin_skills
                .iter()
                .find(|skill| skill.id == selected_builtin_id)
                .cloned()
            {
                let enabled = self.enabled_skill_ids.contains(&skill.id);
                let status_label = if enabled { "Enabled" } else { "Disabled" };
                let status_color = if enabled {
                    gpui::green()
                } else {
                    theme.muted_foreground
                };
                let header = h_flex()
                    .justify_between()
                    .items_start()
                    .mb_6()
                    .child(
                        v_flex()
                            .gap_2()
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
                                    .child(skill.description.clone()),
                            ),
                    )
                    .child(
                        h_flex().gap_2().items_center().child(
                            div()
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .bg(theme.secondary)
                                .text_sm()
                                .text_color(status_color)
                                .child(status_label),
                        ),
                    );

                let details = v_flex()
                    .gap_4()
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_bold()
                                    .text_color(theme.foreground)
                                    .child("Skill Definition"),
                            )
                            .child(
                                v_flex()
                                    .gap_3()
                                    .p_4()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(theme.border)
                                    .bg(theme.background)
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(theme.muted_foreground)
                                                    .child("Skill Name"),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_bold()
                                                    .text_color(theme.foreground)
                                                    .child(skill.name.clone()),
                                            ),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(theme.muted_foreground)
                                                    .child("Description"),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(theme.foreground)
                                                    .child(skill.description.clone()),
                                            ),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(theme.muted_foreground)
                                                    .child("Instruction"),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(theme.foreground)
                                                    .child(skill.instructions.clone()),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_bold()
                                    .text_color(theme.foreground)
                                    .child("LLM Injection Instruction"),
                            )
                            .child(
                                div()
                                    .p_4()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(theme.border)
                                    .bg(theme.secondary)
                                    .text_sm()
                                    .text_color(theme.foreground)
                                    .child(skill.instructions.clone()),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_bold()
                                    .text_color(theme.foreground)
                                    .child("Metadata"),
                            )
                            .child(
                                v_flex()
                                    .gap_2()
                                    .p_4()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(theme.border)
                                    .bg(theme.background)
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(div().text_sm().text_color(theme.muted_foreground).child("ID"))
                                            .child(div().text_sm().text_color(theme.foreground).child(skill.id.clone())),
                                    )
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(div().text_sm().text_color(theme.muted_foreground).child("Category"))
                                            .child(div().text_sm().text_color(theme.foreground).child(skill.category.clone())),
                                    )
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(div().text_sm().text_color(theme.muted_foreground).child("Version"))
                                            .child(div().text_sm().text_color(theme.foreground).child(skill.version.clone())),
                                    )
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(div().text_sm().text_color(theme.muted_foreground).child("Status"))
                                            .child(div().text_sm().text_color(status_color).child(status_label)),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(if enabled {
                                "This skill is enabled and will be included in governed LLM context for the default scope."
                            } else {
                                "This skill is currently disabled and will not be injected into LLM context."
                            }),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_bold()
                                    .text_color(theme.foreground)
                                    .child("Production Readiness Fields"),
                            )
                            .child(
                                v_flex()
                                    .gap_2()
                                    .p_4()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(theme.border)
                                    .bg(theme.background)
                                    .child(div().text_sm().text_color(theme.foreground).child("A complete skill should also define input schema, output schema, trigger conditions, required tools/MCP, permission boundaries, example prompts/results, evaluation benchmark, rollback criteria, owner, and version changelog."))
                                    .child(div().text_xs().text_color(theme.muted_foreground).child("Current built-in skills provide name, description, instruction, category, version and id. The remaining fields should be added before treating a skill as production-governed behavior.")),
                            ),
                    );

                main_content = main_content
                    .child(header)
                    .child(div().h(gpui::px(1.)).w_full().bg(theme.border).my_4())
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scrollbar()
                            .child(details),
                    );
            }
        } else if let Some(selected_id) = &self.selected_skill_id {
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
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(theme.secondary)
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(match skill.status {
                                SkillStatus::Learning => "Review required",
                                SkillStatus::Active => "Linked to execution",
                                SkillStatus::Paused => "Paused",
                            }),
                    );

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
                            .child("Select a learned automation to view its validated workflow"),
                    ),
            );
        }

        h_flex().size_full().child(sidebar).child(main_content)
    }
}

impl EventEmitter<PanelEvent> for SessionPanel {}
