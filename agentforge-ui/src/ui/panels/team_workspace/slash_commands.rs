use gpui_component::IconName;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SlashCommandId {
    Goal,
    Spec,
    Plan,
    Run,
    Review,
    Fix,
    Test,
    Verify,
    Help,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SlashCommandTone {
    Accent,
    Blue,
    Green,
    Yellow,
    Red,
}

#[derive(Clone)]
pub(crate) struct SlashCommandDefinition {
    pub(crate) id: SlashCommandId,
    pub(crate) trigger: &'static str,
    pub(crate) label: &'static str,
    pub(crate) usage: &'static str,
    pub(crate) description: &'static str,
    pub(crate) icon: IconName,
    pub(crate) tone: SlashCommandTone,
    pub(crate) aliases: &'static [&'static str],
    pub(crate) requires_argument: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SlashInvocation {
    pub(crate) command: SlashCommandId,
    pub(crate) argument: String,
    pub(crate) normalized_input: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SlashCommandParseError {
    EmptyCommand,
    UnknownCommand(String),
    MissingArgument(SlashCommandId),
}

impl SlashCommandId {
    pub(crate) fn definition(self) -> SlashCommandDefinition {
        slash_commands()
            .into_iter()
            .find(|command| command.id == self)
            .expect("slash command id must be registered")
    }
}

impl SlashCommandParseError {
    pub(crate) fn message(&self) -> String {
        match self {
            Self::EmptyCommand => slash_help_text(),
            Self::UnknownCommand(command) => format!(
                "Không tìm thấy slash command '/{}'. Gõ /help để xem danh sách command.",
                command
            ),
            Self::MissingArgument(command) => {
                let definition = command.definition();
                format!("Dùng: {}", definition.usage)
            }
        }
    }
}

pub(crate) fn slash_commands() -> Vec<SlashCommandDefinition> {
    vec![
        SlashCommandDefinition {
            id: SlashCommandId::Goal,
            trigger: "goal",
            label: "Goal",
            usage: "/goal <mục tiêu>",
            description: "Chuẩn hóa mục tiêu thành brief, acceptance criteria và constraints",
            icon: IconName::BookOpen,
            tone: SlashCommandTone::Accent,
            aliases: &["brief", "objective"],
            requires_argument: true,
        },
        SlashCommandDefinition {
            id: SlashCommandId::Spec,
            trigger: "spec",
            label: "Spec",
            usage: "/spec <mục tiêu>",
            description: "Tạo đặc tả yêu cầu: scope, API, dữ liệu, edge cases",
            icon: IconName::SquareTerminal,
            tone: SlashCommandTone::Blue,
            aliases: &["design"],
            requires_argument: true,
        },
        SlashCommandDefinition {
            id: SlashCommandId::Plan,
            trigger: "plan",
            label: "Plan",
            usage: "/plan <mục tiêu>",
            description: "Lập kế hoạch thực thi: bước làm, rủi ro, tiêu chí hoàn thành",
            icon: IconName::ChartPie,
            tone: SlashCommandTone::Blue,
            aliases: &["steps"],
            requires_argument: true,
        },
        SlashCommandDefinition {
            id: SlashCommandId::Run,
            trigger: "run",
            label: "Run",
            usage: "/run",
            description: "Chạy các pending tasks đã được phân công cho agent",
            icon: IconName::ArrowRight,
            tone: SlashCommandTone::Green,
            aliases: &["execute"],
            requires_argument: false,
        },
        SlashCommandDefinition {
            id: SlashCommandId::Review,
            trigger: "review",
            label: "Review",
            usage: "/review <phạm vi>",
            description: "Review code hoặc kế hoạch: bugs, rủi ro, test gaps",
            icon: IconName::Search,
            tone: SlashCommandTone::Yellow,
            aliases: &["rev"],
            requires_argument: true,
        },
        SlashCommandDefinition {
            id: SlashCommandId::Fix,
            trigger: "fix",
            label: "Fix",
            usage: "/fix <vấn đề>",
            description: "Sửa lỗi theo mô tả và xác minh lại kết quả",
            icon: IconName::Settings2,
            tone: SlashCommandTone::Red,
            aliases: &["correct"],
            requires_argument: true,
        },
        SlashCommandDefinition {
            id: SlashCommandId::Test,
            trigger: "test",
            label: "Test",
            usage: "/test <phạm vi>",
            description: "Chọn, chạy và tóm tắt test liên quan đến phạm vi",
            icon: IconName::CircleCheck,
            tone: SlashCommandTone::Green,
            aliases: &["tests"],
            requires_argument: true,
        },
        SlashCommandDefinition {
            id: SlashCommandId::Verify,
            trigger: "verify",
            label: "Verify",
            usage: "/verify <thay đổi hoặc kết quả>",
            description: "Kiểm chứng thay đổi, kết quả, rủi ro còn lại",
            icon: IconName::Check,
            tone: SlashCommandTone::Green,
            aliases: &["check"],
            requires_argument: true,
        },
        SlashCommandDefinition {
            id: SlashCommandId::Help,
            trigger: "help",
            label: "Help",
            usage: "/help",
            description: "Hiển thị danh sách slash commands và cách dùng",
            icon: IconName::Info,
            tone: SlashCommandTone::Accent,
            aliases: &["commands", "?"],
            requires_argument: false,
        },
    ]
}

pub(crate) fn filter_slash_commands(query: &str) -> Vec<SlashCommandDefinition> {
    let query = normalize_query(query);
    slash_commands()
        .into_iter()
        .filter(|command| {
            query.is_empty()
                || command.trigger.starts_with(&query)
                || command.label.to_lowercase().starts_with(&query)
                || command
                    .aliases
                    .iter()
                    .any(|alias| alias.starts_with(&query))
        })
        .collect()
}

pub(crate) fn slash_query_from_input(input: &str) -> Option<String> {
    let input = input.trim_start();
    let rest = input.strip_prefix('/')?;
    if rest.chars().any(char::is_whitespace) {
        return None;
    }
    Some(rest.to_string())
}

pub(crate) fn parse_slash_invocation(
    input: &str,
    selected_command: Option<SlashCommandId>,
) -> Result<Option<SlashInvocation>, SlashCommandParseError> {
    let input = input.trim();

    if let Some(command) = selected_command {
        if !input.starts_with('/') {
            return build_invocation(command, input.to_string()).map(Some);
        }
    }

    if input.is_empty() {
        return Ok(None);
    }
    let Some(rest) = input.strip_prefix('/') else {
        return Ok(None);
    };
    if rest.trim().is_empty() {
        return Err(SlashCommandParseError::EmptyCommand);
    }

    let token_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let token = rest[..token_end].trim().to_lowercase();
    let argument = rest[token_end..].trim().to_string();
    let Some(command) = command_by_trigger_or_alias(&token) else {
        return Err(SlashCommandParseError::UnknownCommand(token));
    };

    build_invocation(command.id, argument).map(Some)
}

pub(crate) fn build_slash_prompt(invocation: &SlashInvocation, timestamp: &str) -> Option<String> {
    let goal = invocation.argument.trim();
    match invocation.command {
        SlashCommandId::Goal => Some(format!(
            "Chuẩn hóa mục tiêu sau thành một execution brief có thể giao cho agent:\n{}\n\nYêu cầu:\n- Làm rõ objective, context, in-scope/out-of-scope, constraints và assumptions.\n- Định nghĩa acceptance criteria có thể kiểm chứng.\n- Nêu open questions nếu thiếu thông tin quan trọng.\n- Dùng tool write_file để lưu brief vào docs/goals/goal_{}.md; không xuất thao tác file dưới dạng code block.\n\nBrief phải gồm Objective, Context, Scope, Constraints, Acceptance Criteria, Risks và Open Questions cho mục tiêu:\n{}\n",
            goal, timestamp, goal
        )),
        SlashCommandId::Spec => Some(format!(
            "Tạo đặc tả (spec) cho mục tiêu sau:\n{}\n\nYêu cầu:\n- Spec rõ scope/in-scope/out-of-scope, API/behavior, dữ liệu, edge cases.\n- Nêu open_questions nếu thiếu thông tin.\n- Dùng tool write_file để lưu spec vào docs/specs/spec_{}.md; không xuất thao tác file dưới dạng code block.\n\nSpec phải gồm Objective, Scope, Requirements, API, Data Model, Edge Cases và Open Questions cho mục tiêu:\n{}\n",
            goal, timestamp, goal
        )),
        SlashCommandId::Plan => Some(format!(
            "Tạo kế hoạch thực thi cho mục tiêu sau:\n{}\n\nYêu cầu:\n- Trả về checklist các bước + rủi ro + tiêu chí hoàn thành.\n- Nếu phù hợp, tạo subtasks theo vai trò bằng create_subtasks.\n- Dùng tool write_file để lưu plan vào docs/plans/plan_{}.md; không xuất thao tác file dưới dạng code block.\n\nNội dung plan phải gồm Goal, Plan, Risks và Done cho mục tiêu:\n{}\n",
            goal, timestamp, goal
        )),
        SlashCommandId::Review => Some(format!(
            "Review phạm vi sau theo tư duy code review ưu tiên bugs, regression, rủi ro và test gaps:\n{}\n\nYêu cầu:\n- Kiểm tra source liên quan trước khi kết luận.\n- Findings phải có mức độ nghiêm trọng, file/line nếu có, impact và đề xuất sửa.\n- Nêu counterevidence hoặc lý do nếu không tìm thấy vấn đề.\n- Dùng tool write_file để lưu report vào docs/reviews/review_{}.md; không xuất thao tác file dưới dạng code block.\n\nReport phải gồm Summary, Findings, Test Gaps, Residual Risk và Recommended Next Steps cho phạm vi:\n{}\n",
            goal, timestamp, goal
        )),
        SlashCommandId::Fix => Some(format!(
            "Sửa vấn đề sau trong codebase:\n{}\n\nYêu cầu:\n- Xác định nguyên nhân gốc và phạm vi ảnh hưởng trước khi sửa.\n- Thực hiện thay đổi tối thiểu, phù hợp pattern hiện có.\n- Chạy kiểm tra liên quan hoặc giải thích rõ nếu không chạy được.\n- Sau khi sửa, tóm tắt files thay đổi, verification và rủi ro còn lại.\n\nVấn đề cần sửa:\n{}\n",
            goal, goal
        )),
        SlashCommandId::Test => Some(format!(
            "Thiết kế và chạy kiểm tra cho phạm vi sau:\n{}\n\nYêu cầu:\n- Chọn test/build/check liên quan nhất với thay đổi hoặc hành vi được mô tả.\n- Nếu thiếu test coverage, đề xuất hoặc thêm test phù hợp với mức rủi ro.\n- Tóm tắt command đã chạy, kết quả, lỗi nếu có và bước tiếp theo.\n\nPhạm vi cần test:\n{}\n",
            goal, goal
        )),
        SlashCommandId::Verify => Some(format!(
            "Kiểm chứng thay đổi hoặc kết quả sau:\n{}\n\nYêu cầu:\n- Đối chiếu behavior mong muốn với implementation thực tế.\n- Chạy kiểm tra liên quan nếu có thể.\n- Xác nhận pass/fail rõ ràng, nêu bằng chứng và rủi ro còn lại.\n\nĐối tượng cần verify:\n{}\n",
            goal, goal
        )),
        SlashCommandId::Run | SlashCommandId::Help => None,
    }
}

pub(crate) fn slash_help_text() -> String {
    let commands = slash_commands()
        .into_iter()
        .map(|command| format!("{} - {}", command.usage, command.description))
        .collect::<Vec<_>>()
        .join("\n");
    format!("Slash commands:\n{}", commands)
}

fn build_invocation(
    command: SlashCommandId,
    argument: String,
) -> Result<SlashInvocation, SlashCommandParseError> {
    let definition = command.definition();
    let argument = argument.trim().to_string();
    if definition.requires_argument && argument.is_empty() {
        return Err(SlashCommandParseError::MissingArgument(command));
    }

    let normalized_input = if argument.is_empty() {
        format!("/{}", definition.trigger)
    } else {
        format!("/{} {}", definition.trigger, argument)
    };

    Ok(SlashInvocation {
        command,
        argument,
        normalized_input,
    })
}

fn command_by_trigger_or_alias(token: &str) -> Option<SlashCommandDefinition> {
    slash_commands().into_iter().find(|command| {
        command.trigger == token
            || command.label.eq_ignore_ascii_case(token)
            || command.aliases.iter().any(|alias| *alias == token)
    })
}

fn normalize_query(query: &str) -> String {
    query.trim().trim_start_matches('/').trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slash_command_filter_p_only_returns_plan() {
        let commands = filter_slash_commands("/p");
        let triggers = commands
            .into_iter()
            .map(|command| command.trigger)
            .collect::<Vec<_>>();

        assert_eq!(triggers, vec!["plan"]);
    }

    #[test]
    fn slash_command_filter_r_returns_run_then_review() {
        let commands = filter_slash_commands("/r");
        let triggers = commands
            .into_iter()
            .map(|command| command.trigger)
            .collect::<Vec<_>>();

        assert_eq!(triggers, vec!["run", "review"]);
    }

    #[test]
    fn slash_command_parse_rejects_prefix_collision() {
        let parsed = parse_slash_invocation("/planet build login", None);

        assert_eq!(
            parsed,
            Err(SlashCommandParseError::UnknownCommand("planet".to_string()))
        );
    }

    #[test]
    fn slash_command_parse_selected_command_prepends_trigger() {
        let parsed = parse_slash_invocation("build login", Some(SlashCommandId::Plan))
            .unwrap()
            .unwrap();

        assert_eq!(parsed.command, SlashCommandId::Plan);
        assert_eq!(parsed.argument, "build login");
        assert_eq!(parsed.normalized_input, "/plan build login");
    }

    #[test]
    fn slash_command_argument_rules_are_enforced() {
        let run = parse_slash_invocation("/run", None).unwrap().unwrap();
        assert_eq!(run.command, SlashCommandId::Run);

        assert_eq!(
            parse_slash_invocation("/plan", None),
            Err(SlashCommandParseError::MissingArgument(
                SlashCommandId::Plan
            ))
        );
        assert_eq!(
            parse_slash_invocation("/spec", None),
            Err(SlashCommandParseError::MissingArgument(
                SlashCommandId::Spec
            ))
        );
    }

    #[test]
    fn slash_command_prompts_include_expected_artifacts_or_behavior() {
        let timestamp = "20260625_120000";
        let cases = [
            (SlashCommandId::Goal, "docs/goals/goal_20260625_120000.md"),
            (SlashCommandId::Spec, "docs/specs/spec_20260625_120000.md"),
            (SlashCommandId::Plan, "docs/plans/plan_20260625_120000.md"),
            (
                SlashCommandId::Review,
                "docs/reviews/review_20260625_120000.md",
            ),
            (SlashCommandId::Fix, "Sửa vấn đề"),
            (SlashCommandId::Test, "Thiết kế và chạy kiểm tra"),
            (SlashCommandId::Verify, "Kiểm chứng thay đổi"),
        ];

        for (command, expected) in cases {
            let invocation = SlashInvocation {
                command,
                argument: "build login".to_string(),
                normalized_input: "/cmd build login".to_string(),
            };
            let prompt = build_slash_prompt(&invocation, timestamp).unwrap();
            assert!(
                prompt.contains(expected),
                "prompt for {:?} did not contain {:?}: {}",
                command,
                expected,
                prompt
            );
        }
    }
}
