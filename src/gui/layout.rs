use gpui::*;
use gpui_component::{h_flex, v_flex, IconName};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::{DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_component::progress::Progress;
use gpui_component::slider::{Slider, SliderState};
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

pub fn menu_bar(c: &UiColors, tr: &Tr) -> impl IntoElement {
    h_flex()
        .items_center()
        .w_full()
        .h(px(theme::MENUBAR_HEIGHT))
        .px_2().gap_1()
        .bg(c.control_bar_bg)
        .child(menu_dropdown(tr.menu_file, IconName::Folder, file_menu))
        .child(menu_dropdown(tr.menu_playback, IconName::ChevronRight, playback_menu))
        .child(menu_dropdown(tr.menu_playlist, IconName::SquareTerminal, playlist_menu))
        .child(menu_dropdown(tr.menu_lyric, IconName::BookOpen, lyric_menu))
        .child(menu_dropdown(tr.menu_view, IconName::LayoutDashboard, view_menu))
        .child(menu_dropdown(tr.menu_tools, IconName::Settings, tools_menu))
        .child(menu_dropdown(tr.menu_help, IconName::Info, help_menu))
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
        .ghost()
        .compact()
        .dropdown_menu(build_menu)
}

fn file_menu(menu: PopupMenu, _w: &mut Window, _cx: &mut Context<PopupMenu>) -> PopupMenu {
    menu.item(PopupMenuItem::new("打开文件").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("打开文件夹").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("打开 URL").on_click(|_, _, _| {}))
        .separator()
        .item(PopupMenuItem::new("退出").on_click(|_, _, _| {}))
}

fn playback_menu(menu: PopupMenu, _w: &mut Window, _cx: &mut Context<PopupMenu>) -> PopupMenu {
    menu.item(PopupMenuItem::new("播放/暂停").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("停止").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("上一曲").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("下一曲").on_click(|_, _, _| {}))
        .separator()
        .item(PopupMenuItem::new("快退").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("快进").on_click(|_, _, _| {}))
        .separator()
        .item(PopupMenuItem::new("加速").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("减速").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("原始速度").on_click(|_, _, _| {}))
}

fn playlist_menu(menu: PopupMenu, _w: &mut Window, _cx: &mut Context<PopupMenu>) -> PopupMenu {
    menu.item(PopupMenuItem::new("添加").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("删除").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("重新加载").on_click(|_, _, _| {}))
        .separator()
        .item(PopupMenuItem::new("另存为新播放列表").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("保存当前播放列表").on_click(|_, _, _| {}))
}

fn lyric_menu(menu: PopupMenu, _w: &mut Window, _cx: &mut Context<PopupMenu>) -> PopupMenu {
    menu.item(PopupMenuItem::new("重新加载歌词").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("复制当前歌词").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("编辑歌词").on_click(|_, _, _| {}))
        .separator()
        .item(PopupMenuItem::new("显示歌词翻译").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("显示桌面歌词").on_click(|_, _, _| {}))
}

fn view_menu(menu: PopupMenu, _w: &mut Window, _cx: &mut Context<PopupMenu>) -> PopupMenu {
    menu.item(PopupMenuItem::new("显示播放列表").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("浮动播放列表").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("使用标准标题栏").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("显示菜单栏").on_click(|_, _, _| {}))
        .separator()
        .item(PopupMenuItem::new("迷你模式").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("全屏").on_click(|_, _, _| {}))
        .separator()
        .item(PopupMenuItem::new("深色模式").on_click(|_, _, _| {}))
}

fn tools_menu(menu: PopupMenu, _w: &mut Window, _cx: &mut Context<PopupMenu>) -> PopupMenu {
    menu.item(PopupMenuItem::new("媒体库").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("查找").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("探索路径").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("歌曲信息").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("均衡器").on_click(|_, _, _| {}))
        .separator()
        .item(PopupMenuItem::new("选项设置").on_click(|_, _, _| {}))
}

fn help_menu(menu: PopupMenu, _w: &mut Window, _cx: &mut Context<PopupMenu>) -> PopupMenu {
    menu.item(PopupMenuItem::new("帮助 (F1)").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("在线帮助").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("常见问题").on_click(|_, _, _| {}))
        .separator()
        .item(PopupMenuItem::new("更新日志").on_click(|_, _, _| {}))
        .item(PopupMenuItem::new("支持的格式").on_click(|_, _, _| {}))
        .separator()
        .item(PopupMenuItem::new("关于").on_click(|_, _, _| {}))
}

pub fn main_content(c: &UiColors, tr: &Tr) -> impl IntoElement {
    h_flex()
        .w_full()
        .flex_grow()
        .child(icon_sidebar(c))
        .child(content_area(c, tr))
}

fn icon_sidebar(c: &UiColors) -> impl IntoElement {
    v_flex()
        .items_center()
        .w(px(52.0))
        .h_full()
        .py_2().gap_1()
        .bg(c.panel)
        .child(sidebar_btn("queue", IconName::LayoutDashboard, true, c))
        .child(sidebar_btn("folders", IconName::Folder, false, c))
        .child(sidebar_btn("playlists", IconName::SquareTerminal, false, c))
        .child(sidebar_btn("recent", IconName::Calendar, false, c))
        .child(sidebar_btn("media", IconName::File, false, c))
        .child(div().flex_grow())
        .child(sidebar_btn("search", IconName::Search, false, c))
        .child(sidebar_btn("lyrics", IconName::BookOpen, false, c))
}

fn sidebar_btn(id: &'static str, icon: IconName, active: bool, c: &UiColors) -> impl IntoElement {
    let bg = if active { c.accent } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } };
    Button::new(id)
        .icon(icon)
        .ghost()
        .compact()
        .w(px(40.0)).h(px(40.0))
        .rounded(px(8.0))
        .bg(bg)
}

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

pub fn control_bar(
    c: &UiColors,
    title: &str,
    artist: &str,
    position: f64,
    duration: f64,
    is_playing: bool,
    slider_state: &Entity<SliderState>,
) -> impl IntoElement {
    let pos_str = format!(
        "{:02}:{:02} / {:02}:{:02}",
        (position as u32) / 60,
        (position as u32) % 60,
        (duration as u32) / 60,
        (duration as u32) % 60
    );
    let pct = if duration > 0.0 {
        (position / duration * 100.0) as f32
    } else {
        0.0
    };

    v_flex()
        .w_full()
        .bg(c.control_bar_bg)
        .child(
            h_flex()
                .w_full()
                .h(px(theme::PROGRESS_BAR_HEIGHT))
                .child(Progress::new().value(pct).bg(c.accent).w_full()),
        )
        .child(
            h_flex()
                .items_center()
                .w_full()
                .px_4().gap_4()
                .h(px(theme::CONTROL_BAR_HEIGHT))
                .child(
                    h_flex().items_center().gap_3()
                        .child(Button::new("prev").icon(IconName::ChevronLeft).ghost().compact())
                        .child(Button::new("play").label(if is_playing { "⏸" } else { "▶" }).primary().compact())
                        .child(Button::new("next").icon(IconName::ChevronRight).ghost().compact()),
                )
                .child(
                    v_flex().flex_grow().gap_1()
                        .child(txt(title, 13.0, c.text_title))
                        .child(txt(artist, 11.0, c.text_dim)),
                )
                .child(
                    h_flex().items_center().gap_2()
                        .child(txt(&pos_str, 11.0, c.text_dim))
                        .child(Slider::new(slider_state).horizontal().w(px(96.0))),
                ),
        )
}

pub fn status_bar(c: &UiColors, _tr: &Tr) -> impl IntoElement {
    h_flex()
        .items_center()
        .w_full()
        .h(px(theme::STATUSBAR_HEIGHT))
        .px_3()
        .bg(c.statusbar_bg)
        .child(txt("HackMagic Music Player", 11.0, c.text_dim))
        .child(div().flex_grow())
}
