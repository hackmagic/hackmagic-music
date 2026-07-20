use gpui::*;
use gpui_component::{h_flex, v_flex, IconName};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::{DropdownMenu, PopupMenu};
use crate::gui::theme::{self, UiColors};
use crate::gui::i18n::Tr;

pub fn txt(label: &str, size: f32, color: Hsla) -> impl IntoElement {
    div().text_size(px(size)).text_color(color).child(label.to_string())
}

pub fn title_bar(c: &UiColors, tr: &Tr) -> impl IntoElement {
    h_flex()
        .items_center()
        .w_full()
        .h(px(theme::TITLEBAR_HEIGHT))
        .px_2().gap_2()
        .bg(c.titlebar_bg)
        .child(
            h_flex().items_center().gap_2()
                .child(
                    div()
                        .w(px(28.0)).h(px(28.0)).rounded(px(14.0))
                        .bg(c.accent)
                        .flex().items_center().justify_center()
                        .child(txt("♪", 14.0, gpui::white())),
                )
                .child(txt(tr.app_title, 13.0, c.text)),
        )
        .child(div().flex_grow())
        .child(Button::new("menu").icon(IconName::Menu).ghost().compact())
        .child(Button::new("settings").icon(IconName::Settings).ghost().compact())
        .child(div().w(px(1.0)).h(px(16.0)).bg(c.border))
        .child(Button::new("minimize").icon(IconName::Minimize).ghost().compact())
        .child(Button::new("maximize").icon(IconName::Maximize).ghost().compact())
        .child(Button::new("close").icon(IconName::Close).ghost().compact())
}

pub fn menu_dropdown<F>(
    label: &'static str,
    icon: IconName,
    build_menu: F,
) -> impl IntoElement
where
    F: Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
{
    Button::new(label)
        .icon(icon)
        .label(label)
        .ghost()
        .compact()
        .dropdown_menu(build_menu)
}

/// Fallback content shown for any `Panel` variant not handled elsewhere
/// (effectively a safety net; all real panels are rendered in `MusicPlayer::render`).
pub fn content_area(c: &UiColors, tr: &Tr) -> impl IntoElement {
    v_flex()
        .flex_grow()
        .h_full()
        .bg(c.bg)
        .child(content_header(c, tr))
        .child(playlist_view(c, tr))
}

fn content_header(c: &UiColors, tr: &Tr) -> impl IntoElement {
    h_flex()
        .items_center().justify_between()
        .w_full()
        .px_4().py_3()
        .border_b_1()
        .border_color(c.border)
        .child(txt(tr.pq_title, 18.0, c.text_title))
        .child(
            h_flex().gap_2()
                .child(Button::new("shuffle").label("🔀").ghost().compact())
                .child(Button::new("sort").icon(IconName::SortAscending).ghost().compact()),
        )
}

fn playlist_view(c: &UiColors, tr: &Tr) -> impl IntoElement {
    v_flex()
        .flex_grow()
        .p_4()
        .child(txt(tr.info_open_file, 14.0, c.text_dim))
}
