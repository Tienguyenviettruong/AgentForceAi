import re

with open('/workspace/agentforge-ui/src/ui/panels/team_workspace/chat.rs', 'r') as f:
    content = f.read()

# Let's find the block using regex to be safe
pattern = re.compile(r'let is_image = \["PNG", "JPG", "JPEG", "GIF", "WEBP"\].contains\(&ext\.as_str\(\)\);\s+let icon_box = div\(\).*?cx\.notify\(\);\s+\}\)\)\s+\)\s+\);\s+\}', re.DOTALL)

new_str = """let is_image = ["PNG", "JPG", "JPEG", "GIF", "WEBP"].contains(&ext.as_str());
                            
                            if is_image {
                                use gpui::StyledImage;
                                let path_buf = std::path::PathBuf::from(path);
                                files_container = files_container.child(
                                    div()
                                        .w(px(64.))
                                        .h(px(64.))
                                        .relative()
                                        .rounded_md()
                                        .border_1()
                                        .border_color(theme.border)
                                        .overflow_hidden()
                                        .group(format!("img-upload-{}", idx))
                                        .child(gpui::img(path_buf).w_full().h_full().object_fit(gpui::ObjectFit::Cover))
                                        .child(
                                            div()
                                                .absolute()
                                                .top_1()
                                                .right_1()
                                                .p(px(2.))
                                                .bg(gpui::rgba(0x000000aa))
                                                .rounded_full()
                                                .cursor_pointer()
                                                .invisible()
                                                .group_hover(format!("img-upload-{}", idx), |s| s.visible())
                                                .child(Icon::new(IconName::X).size(px(12.)).text_color(gpui::white()))
                                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                                    this.attached_files.remove(idx);
                                                    cx.notify();
                                                }))
                                        )
                                );
                            } else {
                                let icon_box = div()
                                    .w(px(32.))
                                    .h(px(32.))
                                    .bg(theme.background)
                                    .rounded_sm()
                                    .overflow_hidden()
                                    .flex()
                                    .justify_center()
                                    .items_center()
                                    .child(Icon::new(IconName::File).size(px(14.)).text_color(theme.muted_foreground));

                                files_container = files_container.child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .p_1()
                                        .pr_2()
                                        .rounded_md()
                                        .bg(theme.secondary)
                                        .border_1()
                                        .border_color(theme.border)
                                        .child(icon_box)
                                        .child(
                                            div().flex_col()
                                                .child(div().text_xs().text_color(theme.foreground).child(file_name))
                                                .child(div().text_xs().text_color(theme.muted_foreground).child(format!("{} • {:.1} KB", ext, size_kb)))
                                        )
                                        .child(
                                            div()
                                                .p_1()
                                                .rounded_sm()
                                                .cursor_pointer()
                                                .hover(|s| s.bg(theme.secondary_hover))
                                                .child(Icon::new(IconName::Delete).size(px(12.)).text_color(theme.muted_foreground))
                                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                                    this.attached_files.remove(idx);
                                                    cx.notify();
                                                }))
                                        )
                                );
                            }
                        }"""

match = pattern.search(content)
if match:
    content = content[:match.start()] + new_str + content[match.end():]
    with open('/workspace/agentforge-ui/src/ui/panels/team_workspace/chat.rs', 'w') as f:
        f.write(content)
    print("Success")
else:
    print("Pattern not found")
