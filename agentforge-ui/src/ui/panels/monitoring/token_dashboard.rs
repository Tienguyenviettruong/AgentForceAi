use gpui::{div, App, Context, Focusable, IntoElement, ParentElement, Render, Styled, Window, InteractiveElement};
use gpui_component::scroll::ScrollableElement;
use gpui_component::StyledExt;
use gpui_component::ActiveTheme;
use gpui_component::{
    button::Button,
    v_flex, h_flex
};

pub struct TokenDashboard {
    focus_handle: gpui::FocusHandle,
    daily_tokens: usize,
    tokens_per_agent: Vec<(String, usize)>,
    tokens_per_instance: Vec<(String, usize)>,
    agent_instances: Vec<(String, usize)>,
}

impl TokenDashboard {
    pub fn new(cx: &mut App) -> Self {
        let db = crate::AppState::global(cx).db.clone();
        
        let daily_tokens = db.get_total_daily_tokens().unwrap_or(0);
        let tokens_per_agent = db.get_total_tokens_per_agent().unwrap_or_default();
        let tokens_per_instance = db.get_total_tokens_per_instance().unwrap_or_default();
        let agent_instances = db.get_agent_instance_count().unwrap_or_default();

        Self {
            focus_handle: cx.focus_handle(),
            daily_tokens,
            tokens_per_agent,
            tokens_per_instance,
            agent_instances,
        }
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let db = crate::AppState::global(cx).db.clone();
        self.daily_tokens = db.get_total_daily_tokens().unwrap_or(0);
        self.tokens_per_agent = db.get_total_tokens_per_agent().unwrap_or_default();
        self.tokens_per_instance = db.get_total_tokens_per_instance().unwrap_or_default();
        self.agent_instances = db.get_agent_instance_count().unwrap_or_default();
        cx.notify();
    }
}

fn format_tokens(n: usize) -> String {
    if n >= 1000 {
        format!("{}K", n / 1000)
    } else {
        n.to_string()
    }
}





impl Render for TokenDashboard {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let mut agent_list = v_flex()
            .w_full()
            .border_1()
            .border_color(theme.border)
            .rounded_md();
            
        for (agent_name, tokens) in &self.tokens_per_agent {
            agent_list = agent_list.child(
                h_flex()
                    .p_3()
                    .border_b_1()
                    .border_color(theme.border)
                    .justify_between()
                    .child(div().child(agent_name.clone()))
                    .child(div().font_bold().child(format!("{} tokens", format_tokens(*tokens))))
            );
        }
        if self.tokens_per_agent.is_empty() {
            agent_list = agent_list.child(div().p_3().child("No data available"));
        }

        let mut instance_list = v_flex()
            .w_full()
            .border_1()
            .border_color(theme.border)
            .rounded_md();
            
        for (instance_id, tokens) in &self.tokens_per_instance {
            instance_list = instance_list.child(
                h_flex()
                    .p_3()
                    .border_b_1()
                    .border_color(theme.border)
                    .justify_between()
                    .child(div().child(format!("Instance {}", instance_id)))
                    .child(div().font_bold().child(format!("{} tokens", format_tokens(*tokens))))
            );
        }
        if self.tokens_per_instance.is_empty() {
            instance_list = instance_list.child(div().p_3().child("No data available"));
        }

        let mut participation_list = v_flex()
            .w_full()
            .border_1()
            .border_color(theme.border)
            .rounded_md();
            
        for (agent_name, count) in &self.agent_instances {
            participation_list = participation_list.child(
                h_flex()
                    .p_3()
                    .border_b_1()
                    .border_color(theme.border)
                    .justify_between()
                    .child(div().child(agent_name.clone()))
                    .child(div().font_bold().child(format!("{} instances", count)))
            );
        }
        if self.agent_instances.is_empty() {
            participation_list = participation_list.child(div().p_3().child("No data available"));
        }

        v_flex()
            .size_full()
            .p_4()
            .gap_6()
            .bg(theme.background)
            .child(
                h_flex()
                    .w_full()
                    .justify_end()
                    .items_center()
                    .child(
                        Button::new("btn-refresh-tokens")
                            .label("Refresh")
                            .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)))
                    )
            )
            .child(
                h_flex()
                    .w_full()
                    .gap_4()
                    // Daily Usage Card
                    .child(
                        v_flex()
                            .p_4()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_md()
                            .bg(theme.secondary)
                            .flex_1()
                            .child(div().text_sm().text_color(theme.muted_foreground).child("Daily Token Usage"))
                            .child(div().text_2xl().font_bold().child(format_tokens(self.daily_tokens)).mt_2())
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("Tokens used today")
                                    .mt_1()
                            )
                    )
                    // Monthly Cost Card (Placeholder)
                    .child(
                        v_flex()
                            .p_4()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_md()
                            .bg(theme.secondary)
                            .flex_1()
                            .child(div().text_sm().text_color(theme.muted_foreground).child("Estimated Cost (MTD)"))
                            .child(div().text_2xl().font_bold().child("N/A").mt_2())
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("Pending API Pricing")
                                    .mt_1()
                            )
                    )
                    // API Key Status Card (Placeholder)
                    .child(
                        v_flex()
                            .p_4()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_md()
                            .bg(theme.secondary)
                            .flex_1()
                            .child(div().text_sm().text_color(theme.muted_foreground).child("Active Instances"))
                            .child(div().text_2xl().font_bold().child(self.tokens_per_instance.len().to_string()).mt_2())
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("With token usage")
                                    .mt_1()
                            )
                    )
            )
            .child(
                v_flex()
                    .id("dashboard_scroll")
                    .w_full()
                    .gap_6()
                    .mt_4()
                    .overflow_y_scrollbar()
                    .child(
                        h_flex()
                            .w_full()
                            .gap_4()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_2()
                                    .child(div().font_bold().text_lg().child("Tokens by Agent"))
                                    .child(agent_list)
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_2()
                                    .child(div().font_bold().text_lg().child("Tokens by Instance"))
                                    .child(instance_list)
                            )
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().font_bold().text_lg().child("Agent Participation (Instances)"))
                            .child(participation_list)
                    )
            )
    }
}

