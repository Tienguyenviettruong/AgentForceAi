import re

with open('/workspace/agentforge-ui/src/ui/panels/team_workspace/chat.rs', 'r') as f:
    content = f.read()

# Find the block starting from `let is_image = ...` up to the end of the loop body
pattern = re.compile(r'let is_image = \["PNG", "JPG", "JPEG", "GIF", "WEBP"\].contains\(&ext\.as_str\(\)\);\s*if is_image \{.*?\n\s*cx\.notify\(\);\s*\}\)\)\s*\)\s*\);\s*\}\s*\}', re.DOTALL)

new_str = """let is_image = ["PNG", "JPG", "JPEG", "GIF", "WEBP"].contains(&ext.as_str());
                            
                            let icon_box = div()
                                .w(px(32.))
                                .h(px(32.))
                                .bg(theme.background)
                                .rounded_sm()
                                .overflow_hidden()
                                .flex()
                                .justify_center()
                                .items_center();

                            let icon_child = if is_image {
                                use gpui::StyledImage;
                                let path_buf = std::path::PathBuf::from(path);
                                icon_box.child(gpui::img(path_buf).w_full().h_full().object_fit(gpui::ObjectFit::Cover))
                            } else {
                                icon_box.child(Icon::new(IconName::File).size(px(14.)).text_color(theme.muted_foreground))
                            };

                            files_container = files_container.child(
                                div()
                                    .relative()
                                    .group(format!("file-upload-{}", idx))
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .p_1()
                                    .pr_3() // Give some space on right
                                    .rounded_md()
                                    .bg(theme.secondary)
                                    .border_1()
                                    .border_color(theme.border)
                                    .child(icon_child)
                                    .child(
                                        div().flex_col()
                                            .child(div().text_xs().text_color(theme.foreground).child(file_name))
                                            .child(div().text_xs().text_color(theme.muted_foreground).child(format!("{} • {:.1} KB", ext, size_kb)))
                                    )
                                    .child(
                                        div()
                                            .absolute()
                                            .top(px(-6.))
                                            .right(px(-6.))
                                            .w(px(16.))
                                            .h(px(16.))
                                            .flex()
                                            .justify_center()
                                            .items_center()
                                            .bg(theme.border)
                                            .rounded_full()
                                            .cursor_pointer()
                                            .invisible()
                                            .group_hover(format!("file-upload-{}", idx), |s| s.visible().bg(gpui::rgba(0x000000aa)))
                                            .child(Icon::new(IconName::Close).size(px(10.)).text_color(theme.muted_foreground))
                                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                                this.attached_files.remove(idx);
                                                cx.notify();
                                            }))
                                    )
                            );
                        }"""

match = pattern.search(content)
if match:
    content = content[:match.start()] + new_str + content[match.end():]
    with open('/workspace/agentforge-ui/src/ui/panels/team_workspace/chat.rs', 'w') as f:
        f.write(content)
    print("Success")
else:
    print("Pattern not found")
