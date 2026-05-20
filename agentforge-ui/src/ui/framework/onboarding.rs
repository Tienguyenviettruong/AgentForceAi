use gpui::{div, App, IntoElement, ParentElement, RenderOnce, Styled, Window};

pub struct OnboardingStep {
    pub title: String,
    pub description: String,
}

pub struct OnboardingFlow {
    steps: Vec<OnboardingStep>,
    current_step: usize,
}

impl OnboardingFlow {
    pub fn new(steps: Vec<OnboardingStep>) -> Self {
        Self {
            steps,
            current_step: 0,
        }
    }

    pub fn next_step(&mut self) {
        if self.current_step < self.steps.len() - 1 {
            self.current_step += 1;
        }
    }

    pub fn prev_step(&mut self) {
        if self.current_step > 0 {
            self.current_step -= 1;
        }
    }

    pub fn is_finished(&self) -> bool {
        self.current_step >= self.steps.len() - 1
    }
}

impl RenderOnce for OnboardingFlow {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut d = div().flex().flex_col().gap_2();
        if let Some(step) = self.steps.get(self.current_step) {
            d = d
                .child(div().child(step.title.clone()))
                .child(div().child(step.description.clone()));
        }
        d
    }
}
