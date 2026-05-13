use crate::application::iflow_engine::engine::{
    ExecutionStrategy, WorkflowEngine, WorkflowState, WorkflowStatus,
};
use crate::application::iflow_engine::nodes::{Node, NodeType, WorkflowData};
use crate::core::traits::database::DatabasePort;
use crate::core::models::workflow::WorkflowRecord;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::WindowExt;
use gpui_component::{h_flex, v_flex};
use gpui_component::Sizable;
use gpui_component::dock::{Panel, PanelEvent, TitleStyle};
use gpui_component::{ActiveTheme as _, StyledExt as _, Icon, IconName};
use gpui_component::scroll::ScrollableElement as _;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

const NODE_WIDTH: f32 = 200.0;
const HEADER_HEIGHT: f32 = 32.0;
const PORT_HEIGHT: f32 = 24.0;
const PADDING: f32 = 8.0;

#[derive(Clone, Debug, PartialEq)]
pub struct NodePort {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlowNode {
    pub id: Uuid,
    pub node_data: NodeType,
    pub title: String,
    pub position: Point<f32>,
    pub inputs: Vec<NodePort>,
    pub outputs: Vec<NodePort>,
}

impl FlowNode {
    fn input_port_pos(&self, port_id: &str) -> Option<Point<f32>> {
        self.inputs.iter().position(|p| p.id == port_id).map(|idx| {
            point(
                self.position.x + PADDING + 5.0,
                self.position.y
                    + HEADER_HEIGHT
                    + PADDING
                    + (idx as f32 * PORT_HEIGHT)
                    + PORT_HEIGHT / 2.0,
            )
        })
    }

    fn output_port_pos(&self, port_id: &str) -> Option<Point<f32>> {
        self.outputs
            .iter()
            .position(|p| p.id == port_id)
            .map(|idx| {
                let offset_y = HEADER_HEIGHT
                    + PADDING
                    + (self.inputs.len() as f32 * PORT_HEIGHT)
                    + (idx as f32 * PORT_HEIGHT)
                    + PORT_HEIGHT / 2.0;
                point(self.position.x + NODE_WIDTH - PADDING - 5.0, self.position.y + offset_y)
            })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Connection {
    pub id: Uuid,
    pub from_node: Uuid,
    pub from_port: String,
    pub to_node: Uuid,
    pub to_port: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct FlowState {
    pub nodes: Vec<FlowNode>,
    pub connections: Vec<Connection>,
}

pub struct IFlowBuilderPanel {
    focus_handle: FocusHandle,
    state: FlowState,
    undo_stack: Vec<FlowState>,
    redo_stack: Vec<FlowState>,

    zoom: f32,
    pan: Point<f32>,

    dragging_node: Option<(Uuid, Point<f32>)>,
    panning: Option<Point<f32>>,
    connecting: Option<(Uuid, String, Point<f32>)>,

    workflow_engine: WorkflowEngine,
    workflow_id: Option<String>,
    execution_id: Option<String>,
    last_execution_state: Option<WorkflowState>,
    workflow_to_canvas: HashMap<String, Uuid>,
    canvas_to_workflow: HashMap<Uuid, String>,
    
    db: Arc<dyn DatabasePort>,
}

impl IFlowBuilderPanel {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        let db = crate::AppState::global(cx).db.clone();
        
        let workflow_engine = WorkflowEngine::new();
        let workflow = crate::application::iflow_engine::engine::Workflow {
            id: uuid::Uuid::new_v4().to_string(),
            name: "New Workflow".to_string(),
            version: "1.0.0".to_string(),
            nodes: HashMap::new(),
            instance_id: None,
            team_id: None,
            start_node_id: "start".to_string(),
        };
        let workflow_id = workflow.id.clone();
        workflow_engine.register_workflow(workflow.clone());
        let (state, workflow_to_canvas, canvas_to_workflow) =
            Self::build_canvas_from_workflow(&workflow);

        Self {
            focus_handle: cx.focus_handle(),
            state,
            undo_stack: vec![],
            redo_stack: vec![],
            zoom: 1.0,
            pan: point(0.0, 0.0),
            dragging_node: None,
            panning: None,
            connecting: None,
            workflow_engine,
            workflow_id: Some(workflow_id),
            execution_id: None,
            last_execution_state: None,
            workflow_to_canvas,
            canvas_to_workflow,
            db,
        }
    }

    fn load_workflow_record(&mut self, record: &WorkflowRecord) {
        if let Ok(wf) = WorkflowEngine::parse_workflow(&record.definition) {
            self.workflow_engine.register_workflow(wf.clone());
            self.workflow_id = Some(wf.id.clone());
            let (state, workflow_to_canvas, canvas_to_workflow) =
                Self::build_canvas_from_workflow(&wf);
            self.state = state;
            self.workflow_to_canvas = workflow_to_canvas;
            self.canvas_to_workflow = canvas_to_workflow;
            self.execution_id = None;
            self.last_execution_state = None;
        }
    }

    fn build_canvas_from_workflow(
        workflow: &crate::application::iflow_engine::engine::Workflow,
    ) -> (FlowState, HashMap<String, Uuid>, HashMap<Uuid, String>) {
        let mut state = FlowState::default();
        let mut workflow_to_canvas: HashMap<String, Uuid> = HashMap::new();
        let mut canvas_to_workflow: HashMap<Uuid, String> = HashMap::new();

        // Topologically sort nodes starting from "start" (or the first node)
        let mut sorted = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        
        // Find start node, or just use the first available
        if let Some(start) = workflow.nodes.values().find(|n| n.id == "start" || n.node_type == crate::application::iflow_engine::nodes::NodeType::Start) {
            queue.push_back(start.id.clone());
        } else if let Some(first) = workflow.nodes.keys().next() {
            queue.push_back(first.clone());
        }

        while let Some(node_id) = queue.pop_front() {
            if !visited.contains(&node_id) {
                visited.insert(node_id.clone());
                if let Some(node) = workflow.nodes.get(&node_id) {
                    sorted.push(node);
                    for next_id in &node.next_nodes {
                        queue.push_back(next_id.clone());
                    }
                    // Also handle decision nodes
                    if let crate::application::iflow_engine::nodes::NodeType::Decision { true_next, false_next, .. } = &node.node_type {
                        queue.push_back(true_next.clone());
                        queue.push_back(false_next.clone());
                    }
                }
            }
        }

        // Add any disconnected nodes
        for node in workflow.nodes.values() {
            if !visited.contains(&node.id) {
                sorted.push(node);
            }
        }

        for (idx, node) in sorted.iter().enumerate() {
            let canvas_id = Uuid::new_v4();
            workflow_to_canvas.insert(node.id.clone(), canvas_id);
            canvas_to_workflow.insert(canvas_id, node.id.clone());

            let flow_node = FlowNode {
                id: canvas_id,
                node_data: node.node_type.clone(),
                title: node.name.clone(),
                position: point(100.0 + (idx as f32 * 260.0), 120.0),
                inputs: vec![NodePort {
                    id: "in".to_string(),
                    name: "In".to_string(),
                }],
                outputs: vec![NodePort {
                    id: "out".to_string(),
                    name: "Out".to_string(),
                }],
            };
            state.nodes.push(flow_node);
        }

        for node in workflow.nodes.values() {
            if let Some(&from_canvas) = workflow_to_canvas.get(&node.id) {
                for next in &node.next_nodes {
                    if let Some(&to_canvas) = workflow_to_canvas.get(next) {
                        state.connections.push(Connection {
                            id: Uuid::new_v4(),
                            from_node: from_canvas,
                            from_port: "out".to_string(),
                            to_node: to_canvas,
                            to_port: "in".to_string(),
                        });
                    }
                }
            }
        }

        (state, workflow_to_canvas, canvas_to_workflow)
    }

    fn save_state(&mut self) {
        self.undo_stack.push(self.state.clone());
        self.redo_stack.clear();
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.state.clone());
            self.state = prev;
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.state.clone());
            self.state = next;
        }
    }

    fn add_node(&mut self, node_type_str: &str) {
        self.save_state();
        let node_data = match node_type_str {
            "Start" => NodeType::Start,
            "Cron" => NodeType::CronTrigger { interval_ms: 60000 },
            "End" => NodeType::End,
            "Agent Task" => NodeType::AgentTask {
                agent_id: "default_agent".to_string(),
                instruction: "New Instruction".to_string(),
                input_vars: vec![],
                output_var: None,
            },
            "System Command" => NodeType::SystemCommand {
                command: "echo 'hello'".to_string(),
                output_var: None,
            },
            "HTTP Request" => NodeType::HttpRequest {
                method: "GET".to_string(),
                url: "https://api.example.com".to_string(),
                body_var: None,
                output_var: None,
            },
            "Transform" => NodeType::Transform {
                input_var: "in".to_string(),
                output_var: "out".to_string(),
                mode: crate::application::iflow_engine::nodes::TransformMode::Identity,
            },
            "Decision" => NodeType::Decision {
                condition_var: "cond".to_string(),
                true_next: "".to_string(),
                false_next: "".to_string(),
            },
            "Human Review" => NodeType::HumanReview {
                prompt: "Please review".to_string(),
                approved_next: "".to_string(),
                rejected_next: "".to_string(),
                output_var: "review_out".to_string(),
            },
            "Merge" => NodeType::Merge,
            "Delay" => NodeType::Delay { duration_ms: 1000 },
            _ => NodeType::Start,
        };

        let node = FlowNode {
            id: Uuid::new_v4(),
            node_data,
            title: format!("New {}", node_type_str),
            position: point(
                (200.0 - self.pan.x) / self.zoom,
                (200.0 - self.pan.y) / self.zoom,
            ),
            inputs: vec![NodePort {
                id: "in".to_string(),
                name: "In".to_string(),
            }],
            outputs: vec![NodePort {
                id: "out".to_string(),
                name: "Out".to_string(),
            }],
        };
        self.state.nodes.push(node);
    }

    fn start_execution(&mut self, strategy: ExecutionStrategy) -> Result<(), String> {
        let workflow_id = self.workflow_id.clone().ok_or("No workflow")?;
        let exec_id =
            self.workflow_engine
                .start_workflow(&workflow_id, strategy, WorkflowData::new())?;
        self.execution_id = Some(exec_id.clone());
        self.last_execution_state = self.workflow_engine.get_state(&exec_id);
        Ok(())
    }

    fn resolve_review(&mut self, approved: bool) -> Result<(), String> {
        let exec_id = self.execution_id.clone().ok_or("No execution")?;
        let state = self.workflow_engine.resolve_review(&exec_id, approved)?;
        self.last_execution_state = Some(state);
        Ok(())
    }

    fn render_node(&self, node: &FlowNode, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let zoom = self.zoom;
        let pan = self.pan;

        let pos = node.position;
        let left = px(pos.x * zoom + pan.x);
        let top = px(pos.y * zoom + pan.y);
        let width = px(NODE_WIDTH * zoom);

        let node_id = node.id;
        let node_bg = self
            .canvas_to_workflow
            .get(&node_id)
            .and_then(|wid| self.last_execution_state.as_ref().map(|s| (wid, s)))
            .map(|(wid, s)| {
                if s.completed_nodes.contains(wid) {
                    theme.success.opacity(0.15)
                } else if s.current_nodes.iter().any(|n| n == wid) {
                    theme.primary.opacity(0.15)
                } else if s.pending_review.as_deref() == Some(wid.as_str()) {
                    theme.warning.opacity(0.15)
                } else {
                    theme.background
                }
            })
            .unwrap_or(theme.background);

        let border_col = self
            .canvas_to_workflow
            .get(&node_id)
            .and_then(|wid| self.last_execution_state.as_ref().map(|s| (wid, s)))
            .map(|(wid, s)| {
                if s.current_nodes.iter().any(|n| n == wid) {
                    theme.primary
                } else {
                    theme.border
                }
            })
            .unwrap_or(theme.border);

        div()
            .absolute()
            .left(left)
            .top(top)
            .w(width)
            .bg(node_bg)
            .border_2()
            .border_color(border_col)
            .rounded_xl()
            .shadow_md()
            .child(
                div()
                    .bg(theme.background)
                    .h(px(HEADER_HEIGHT * zoom))
                    .p(px(PADDING * zoom))
                    .rounded_t_xl()
                    .border_b_1()
                    .border_color(theme.border)
                    .text_size(px(14.0 * zoom))
                    .font_weight(FontWeight::BOLD)
                    .child(node.title.clone())
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, e: &MouseDownEvent, _window, _cx| {
                            let ex: f32 = e.position.x.into();
                            let ey: f32 = e.position.y.into();
                            let lx: f32 = left.into();
                            let ty: f32 = top.into();
                            this.dragging_node = Some((node_id, point(ex - lx, ey - ty)));
                        }),
                    ),
            )
            .child(
                div()
                    .p(px(PADDING * zoom))
                    .flex()
                    .flex_col()
                    .children(node.inputs.iter().map(|port| {
                        let port_id = port.id.clone();
                        div()
                            .h(px(PORT_HEIGHT * zoom))
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .w(px(12.0 * zoom))
                                    .h(px(12.0 * zoom))
                                    .rounded_full()
                                    .bg(theme.success)
                                    .border_1()
                                    .border_color(theme.background)
                                    .mr(px(8.0 * zoom))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(move |this, _e: &MouseUpEvent, _window, cx| {
                                            if let Some((from_node, from_port, _)) =
                                                this.connecting.clone()
                                            {
                                                this.save_state();
                                                this.state.connections.push(Connection {
                                                    id: Uuid::new_v4(),
                                                    from_node,
                                                    from_port,
                                                    to_node: node_id,
                                                    to_port: port_id.clone(),
                                                });
                                                this.connecting = None;
                                                cx.notify();
                                            }
                                        }),
                                    ),
                            )
                            .child(div().text_size(px(12.0 * zoom)).text_color(theme.muted_foreground).child(port.name.clone()))
                    }))
                    .children(node.outputs.iter().map(|port| {
                        let port_id = port.id.clone();
                        div()
                            .h(px(PORT_HEIGHT * zoom))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(div().text_size(px(12.0 * zoom)).text_color(theme.muted_foreground).child(port.name.clone()))
                            .child(
                                div()
                                    .w(px(12.0 * zoom))
                                    .h(px(12.0 * zoom))
                                    .rounded_full()
                                    .bg(theme.primary)
                                    .border_1()
                                    .border_color(theme.background)
                                    .ml(px(8.0 * zoom))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(
                                            move |this, e: &MouseDownEvent, _window, _cx| {
                                                let ex: f32 = e.position.x.into();
                                                let ey: f32 = e.position.y.into();
                                                this.connecting =
                                                    Some((node_id, port_id.clone(), point(ex, ey)));
                                            },
                                        ),
                                    ),
                            )
                    })),
            )
    }

    fn render_dashboard(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let status_text = self
            .last_execution_state
            .as_ref()
            .map(|s| match &s.status {
                WorkflowStatus::Pending => "Pending".to_string(),
                WorkflowStatus::Running => "Running".to_string(),
                WorkflowStatus::Paused => "Paused".to_string(),
                WorkflowStatus::Completed => "Completed".to_string(),
                WorkflowStatus::Failed(e) => format!("Failed: {}", e),
            })
            .unwrap_or_else(|| "Idle".to_string());
        let pending_review = self
            .last_execution_state
            .as_ref()
            .and_then(|s| s.pending_review.clone());

        div()
            .w_full()
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .shadow_sm()
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .text_size(px(16.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.foreground).child("Dashboard"),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(div().text_color(theme.foreground).child(format!("Nodes: {}", self.state.nodes.len())))
                    .child(div().text_color(theme.foreground).child(format!("Connections: {}", self.state.connections.len()))),
            )
            .child(div().text_color(theme.foreground).child(format!("Execution: {}", status_text)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .bg(theme.muted)
                            .p_2()
                            .rounded_md()
                            .child("Run Serial")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    let _ = this.start_execution(ExecutionStrategy::Serial);
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .bg(theme.muted)
                            .p_2()
                            .rounded_md()
                            .child("Run Parallel")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    let _ = this.start_execution(ExecutionStrategy::Parallel);
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .bg(theme.muted)
                            .p_2()
                            .rounded_md()
                            .child("Step")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    let exec_id = match this.execution_id.clone() {
                                        Some(v) => v,
                                        None => return,
                                    };
                                    let engine = this.workflow_engine.clone();
                                    cx.spawn(async move |this, cx| {
                                        if let Ok(state) = engine.step_execution(&exec_id).await {
                                            let _ = this.update(cx, |this, cx| {
                                                this.last_execution_state = Some(state);
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .detach();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .bg(theme.muted)
                            .p_2()
                            .rounded_md()
                            .child(format!(
                                "Approve{}",
                                pending_review
                                    .as_ref()
                                    .map(|v| format!(" ({})", v))
                                    .unwrap_or_default()
                            ))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    if this
                                        .last_execution_state
                                        .as_ref()
                                        .and_then(|s| s.pending_review.as_ref())
                                        .is_some()
                                    {
                                        let _ = this.resolve_review(true);
                                        cx.notify();
                                    }
                                }),
                            ),
                    )
                    .child(
                        div()
                            .bg(theme.muted)
                            .p_2()
                            .rounded_md()
                            .child("Reject")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    if this
                                        .last_execution_state
                                        .as_ref()
                                        .and_then(|s| s.pending_review.as_ref())
                                        .is_some()
                                    {
                                        let _ = this.resolve_review(false);
                                        cx.notify();
                                    }
                                }),
                            ),
                    ),
            )
    }

    fn render_logs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        
        let mut log_items = vec![];
        log_items.push(div().text_color(theme.muted_foreground).child("• Initialize Environment..."));
        log_items.push(div().text_color(theme.muted_foreground).child("• Ready for execution..."));
        
        if let Some(state) = &self.last_execution_state {
            for node_id in &state.completed_nodes {
                let name = self.canvas_to_workflow.iter()
                    .find(|(_, wid)| *wid == node_id)
                    .and_then(|(cid, _)| self.state.nodes.iter().find(|n| n.id == *cid))
                    .map(|n| n.title.clone())
                    .unwrap_or_else(|| node_id.clone());
                log_items.push(div().text_color(theme.success).child(format!("✓ Completed: {}", name)));
            }
            for node_id in &state.current_nodes {
                let name = self.canvas_to_workflow.iter()
                    .find(|(_, wid)| *wid == node_id)
                    .and_then(|(cid, _)| self.state.nodes.iter().find(|n| n.id == *cid))
                    .map(|n| n.title.clone())
                    .unwrap_or_else(|| node_id.clone());
                log_items.push(div().text_color(theme.primary).child(format!("> Running: {}", name)));
            }
        }
        
        log_items.push(
            h_flex().gap_2().mt_4().items_center()
                
                .child(div().text_color(theme.primary).child("Waiting for AI response..."))
        );

        div()
            .w_full()
            .h_full()
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .text_size(px(18.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.foreground)
                    .child("Execution Logs")
            )
            .child(
                div()
                    .flex_1()
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .children(log_items)
            )
    }

    fn render_palette(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        div()
            .w_full()
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .shadow_sm()
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .text_size(px(16.))
                    .font_weight(FontWeight::BOLD)
                    .child("Palette"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .bg(theme.muted)
                            .p_2()
                            .rounded_md()
                            .child("Add Start")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _e: &MouseDownEvent, _w, _cx| {
                                    this.add_node("Start");
                                }),
                            ),
                    )
                    .child(
                        div()
                            .bg(theme.muted)
                            .p_2()
                            .rounded_md()
                            .child("Add Agent Task")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _e: &MouseDownEvent, _w, _cx| {
                                    this.add_node("Agent Task");
                                }),
                            ),
                    )
                    .child(
                        div()
                            .bg(theme.muted)
                            .p_2()
                            .rounded_md()
                            .child("Add Decision")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _e: &MouseDownEvent, _w, _cx| {
                                    this.add_node("Decision");
                                }),
                            ),
                    )
                    .child(
                        div()
                            .bg(theme.muted)
                            .p_2()
                            .rounded_md()
                            .child("Add Human Review")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _e: &MouseDownEvent, _w, _cx| {
                                    this.add_node("Human Review");
                                }),
                            ),
                    )
                    .child(
                        div()
                            .bg(theme.muted)
                            .p_2()
                            .rounded_md()
                            .child("Add Transform")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _e: &MouseDownEvent, _w, _cx| {
                                    this.add_node("Transform");
                                }),
                            ),
                    )
                    .child(
                        div()
                            .bg(theme.muted)
                            .p_2()
                            .rounded_md()
                            .child("Add Merge")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _e: &MouseDownEvent, _w, _cx| {
                                    this.add_node("Merge");
                                }),
                            ),
                    )
                    .child(
                        div()
                            .bg(theme.muted)
                            .p_2()
                            .rounded_md()
                            .child("Add Delay")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _e: &MouseDownEvent, _w, _cx| {
                                    this.add_node("Delay");
                                }),
                            ),
                    )
                    .child(
                        div()
                            .bg(theme.muted)
                            .p_2()
                            .rounded_md()
                            .child("Add End")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _e: &MouseDownEvent, _w, _cx| {
                                    this.add_node("End");
                                }),
                            ),
                    )
                    .child(
                        div()
                            .bg(theme.muted)
                            .p_2()
                            .rounded_md()
                            .child("Undo")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _e: &MouseDownEvent, _w, _cx| {
                                    this.undo();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .bg(theme.muted)
                            .p_2()
                            .rounded_md()
                            .child("Redo")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _e: &MouseDownEvent, _w, _cx| {
                                    this.redo();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .bg(theme.primary) // blue for action
                            .p_2()
                            .rounded_md()
                            .child("Save Workflow")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _e: &MouseDownEvent, _w, cx| {
                                    this.save_workflow(cx);
                                }),
                            ),
                    ),
            )
    }

    fn save_workflow(&mut self, _cx: &mut Context<Self>) {
        let workflow = self.serialize_to_workflow();
        
        let json = serde_json::to_string(&workflow).unwrap_or_default();
        let record = WorkflowRecord {
            id: workflow.id.clone(),
            name: workflow.name.clone(),
            definition: json,
            version: workflow.version.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        if let Err(e) = self.db.upsert_workflow(&record) {
            eprintln!("Failed to save workflow: {}", e);
        } else {
            println!("Workflow saved successfully to DB.");
            // Update the internal engine too
            self.workflow_engine.register_workflow(workflow);
        }
    }

    fn serialize_to_workflow(&self) -> crate::application::iflow_engine::engine::Workflow {
        let mut nodes_map = HashMap::new();
        let mut start_id = "start".to_string();

        // Pass 1: Build nodes
        for canvas_node in &self.state.nodes {
            let wf_id = self.canvas_to_workflow.get(&canvas_node.id).cloned().unwrap_or_else(|| canvas_node.id.to_string());
            
            if matches!(canvas_node.node_data, NodeType::Start) {
                start_id = wf_id.clone();
            }

            let node = Node {
                id: wf_id.clone(),
                name: canvas_node.title.clone(),
                node_type: canvas_node.node_data.clone(),
                next_nodes: vec![], // populated in pass 2
            };
            nodes_map.insert(wf_id, node);
        }

        // Pass 2: Build connections
        for conn in &self.state.connections {
            let from_wf_id = self.canvas_to_workflow.get(&conn.from_node).cloned().unwrap_or_else(|| conn.from_node.to_string());
            let to_wf_id = self.canvas_to_workflow.get(&conn.to_node).cloned().unwrap_or_else(|| conn.to_node.to_string());
            
            if let Some(node) = nodes_map.get_mut(&from_wf_id) {
                // If it's a decision or review node, we should ideally map ports to true_next/false_next
                match &mut node.node_type {
                    NodeType::Decision { true_next, false_next, .. } => {
                        if conn.from_port == "true" {
                            *true_next = to_wf_id.clone();
                        } else {
                            *false_next = to_wf_id.clone();
                        }
                    }
                    NodeType::HumanReview { approved_next, rejected_next, .. } => {
                        if conn.from_port == "approve" {
                            *approved_next = to_wf_id.clone();
                        } else {
                            *rejected_next = to_wf_id.clone();
                        }
                    }
                    _ => {
                        node.next_nodes.push(to_wf_id);
                    }
                }
            }
        }

        let wf_id = self.workflow_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());

        crate::application::iflow_engine::engine::Workflow {
            id: wf_id,
            name: "Custom Workflow".to_string(),
            version: "1.0.0".to_string(),
            nodes: nodes_map,
            start_node_id: start_id,
            team_id: Some("sdg-team-123".to_string()),
            instance_id: Some("sdg-instance-123".to_string()),
        }
    }

    fn render_minimap(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        div()
            .absolute()
            .bottom(px(16.))
            .right(px(16.))
            .w(px(200.))
            .h(px(150.))
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .shadow_sm()
            .child(
                div()
                    .absolute()
                    .top(px(8.))
                    .left(px(8.))
                    .text_size(px(12.))
                    .child("Minimap"),
            )
            .children(self.state.nodes.iter().map(|n| {
                div()
                    .absolute()
                    .left(px(n.position.x * 0.1 + 100.))
                    .top(px(n.position.y * 0.1 + 75.))
                    .w(px(NODE_WIDTH * 0.1))
                    .h(px(HEADER_HEIGHT * 0.1))
                    .bg(theme.muted_foreground)
            }))
    }
}

impl Panel for IFlowBuilderPanel {
    fn panel_name(&self) -> &'static str {
        "iFlow Builder"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.panel_name()
    }

    fn title_style(&self, _cx: &App) -> Option<TitleStyle> {
        None
    }
}

impl Focusable for IFlowBuilderPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for IFlowBuilderPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let zoom = self.zoom;
        let pan = self.pan;

        let nodes = self.state.nodes.clone();
        let view = cx.entity().clone();

        let mut lines = vec![];
        for conn in &self.state.connections {
            let from_node = nodes.iter().find(|n| n.id == conn.from_node);
            let to_node = nodes.iter().find(|n| n.id == conn.to_node);

            if let (Some(f), Some(t)) = (from_node, to_node) {
                if let (Some(p1), Some(p2)) = (
                    f.output_port_pos(&conn.from_port),
                    t.input_port_pos(&conn.to_port),
                ) {
                    lines.push((p1, p2, false));
                }
            }
        }

        if let Some((from_node, from_port, current_pos)) = &self.connecting {
            if let Some(f) = nodes.iter().find(|n| n.id == *from_node) {
                if let Some(p1) = f.output_port_pos(from_port) {
                    lines.push((p1, *current_pos, true));
                }
            }
        }

        let canvas_panel = div()
            .flex_1()
            .h_full()
            .bg(theme.background)
            .overflow_hidden()
            .relative()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, e: &MouseDownEvent, _window, _cx| {
                    if this.dragging_node.is_none() && this.connecting.is_none() {
                        let ex: f32 = e.position.x.into();
                        let ey: f32 = e.position.y.into();
                        this.panning = Some(point(ex, ey));
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, _e: &MouseUpEvent, _window, cx| {
                    this.dragging_node = None;
                    this.panning = None;
                    this.connecting = None;
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(move |this, e: &MouseMoveEvent, _window, cx| {
                let ex: f32 = e.position.x.into();
                let ey: f32 = e.position.y.into();
                if let Some((node_id, offset)) = this.dragging_node {
                    if let Some(node) = this.state.nodes.iter_mut().find(|n| n.id == node_id) {
                        node.position = point(
                            (ex - offset.x - this.pan.x) / this.zoom,
                            (ey - offset.y - this.pan.y) / this.zoom,
                        );
                        cx.notify();
                    }
                } else if let Some(last_pos) = this.panning {
                    let dx = ex - last_pos.x;
                    let dy = ey - last_pos.y;
                    this.pan.x += dx;
                    this.pan.y += dy;
                    this.panning = Some(point(ex, ey));
                    cx.notify();
                } else if let Some((from_node, from_port, _)) = &this.connecting {
                    this.connecting = Some((
                        *from_node,
                        from_port.clone(),
                        point((ex - this.pan.x) / this.zoom, (ey - this.pan.y) / this.zoom),
                    ));
                    cx.notify();
                }
            }))
            .on_scroll_wheel(cx.listener(move |this, e: &ScrollWheelEvent, _window, cx| {
                let old_zoom = this.zoom;
                let delta = match e.delta {
                    gpui::ScrollDelta::Pixels(p) => {
                        let py: f32 = p.y.into();
                        py
                    }
                    gpui::ScrollDelta::Lines(l) => l.y * 20.0,
                };

                this.zoom += delta * 0.001;
                this.zoom = this.zoom.clamp(0.1, 5.0);

                let mouse_x: f32 = e.position.x.into();
                let mouse_y: f32 = e.position.y.into();

                let zoom_ratio = this.zoom / old_zoom;
                this.pan.x = mouse_x - (mouse_x - this.pan.x) * zoom_ratio;
                this.pan.y = mouse_y - (mouse_y - this.pan.y) * zoom_ratio;

                cx.notify();
            }))
            .child({
                let active_color = theme.foreground.opacity(0.8);
                let inactive_color = theme.foreground.opacity(0.3);
                canvas(
                    move |_bounds, _window, _cx| {},
                    move |bounds, _, window, _cx| {
                        for (p1, p2, is_active) in lines {
                            let start_x = px(p1.x * zoom + pan.x) + bounds.origin.x;
                            let start_y = px(p1.y * zoom + pan.y) + bounds.origin.y;
                            let end_x = px(p2.x * zoom + pan.x) + bounds.origin.x;
                            let end_y = px(p2.y * zoom + pan.y) + bounds.origin.y;

                            let dx: f32 = end_x.into();
                            let sx: f32 = start_x.into();
                            let d_width = px((dx - sx).abs() * 0.5);
                            let color = if is_active {
                                active_color
                            } else {
                                inactive_color
                            };

                            let mut builder = gpui::PathBuilder::stroke(px(2.0)).with_style(
                                gpui::PathStyle::Stroke(gpui::StrokeOptions::default()),
                            );
                            builder.move_to(point(start_x, start_y));
                            builder.cubic_bezier_to(
                                point(end_x, end_y),
                                point(start_x + d_width, start_y),
                                point(end_x - d_width, end_y),
                            );
                            if let Ok(path) = builder.build() {
                                window.paint_path(path, color);
                            }
                        }
                    },
                )
                .absolute()
                .size_full()
            })
            .children(nodes.iter().map(|n| self.render_node(n, cx)))
            .child(self.render_minimap(cx));

        let left_panel = v_flex()
            .w(px(280.))
            .h_full()
            .bg(theme.background)
            .border_r_1()
            .border_color(theme.border)
            .p_4()
            .gap_4()
            .child(
                div()
                    .text_size(px(18.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.foreground).child("Workflow Pipeline")
            )
            .child(self.render_dashboard(cx)).child(self.render_palette(cx))
            .child(
                h_flex().gap_2()
                    .child(
                        Button::new("iflow-load-latest")
                            .small()
                            .label("Load Latest")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let workflows = this.db.list_workflows().unwrap_or_default();
                                if let Some(wf) = workflows.first() {
                                    this.load_workflow_record(wf);
                                    cx.notify();
                                } else {
                                    window.push_notification(
                                        (gpui_component::notification::NotificationType::Info, "No workflows in DB."),
                                        cx,
                                    );
                                }
                            })),
                    )
                    .child(
                        Button::new("iflow-pick")
                            .small()
                            .label("Pick Workflow")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let workflows = this.db.list_workflows().unwrap_or_default();
                                let workflows = std::sync::Arc::new(workflows);
                                let view = view.clone();
                                window.open_dialog(cx, move |dialog, _window, _cx| {
                                    let mut list = v_flex().gap(px(6.)).p(px(12.)).w_full();
                                    for wf in workflows.iter().take(30) {
                                        let wf_id = wf.id.clone();
                                        let wf_name = wf.name.clone();
                                        let wf_clone = wf.clone();
                                        list = list.child(
                                            Button::new(gpui::SharedString::from(format!("pick-{}", wf_id)))
                                                .label(wf_name)
                                                .on_click({
                                                    let view = view.clone();
                                                    move |_, window, cx| {
                                                        view.update(cx, |this: &mut IFlowBuilderPanel, cx| {
                                                            this.load_workflow_record(&wf_clone);
                                                            cx.notify();
                                                        });
                                                        window.close_dialog(cx);
                                                    }
                                                }),
                                        );
                                    }
                                    dialog
                                        .title("Select Workflow")
                                        .w(px(640.))
                                        .child(list)
                                        .footer(|_, _, _, _| {
                                            vec![
                                                Button::new("close-iflow-pick")
                                                    .label("Close")
                                                    .on_click(|_, window, cx| window.close_dialog(cx))
                                                    .into_any_element(),
                                            ]
                                        })
                                });
                            })),
                    )
            );

        let right_panel = v_flex()
            .w(px(280.))
            .h_full()
            .bg(theme.background)
            .border_l_1()
            .border_color(theme.border)
            .p_4()
            .gap_4()
            .child(
                div()
                    .text_size(px(18.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.foreground).child("Node Library")
            )
            .child(self.render_logs(cx));

        h_flex()
            .size_full()
            .bg(theme.background)
            .child(left_panel)
            .child(canvas_panel)
            .child(right_panel)
    }
}

impl EventEmitter<PanelEvent> for IFlowBuilderPanel {}
