# GPUI API Reference — Worked Examples

Verified against `gpui 0.2.2` + `gpui-component 0.5.1` in this project.

## Scrollable floating playlist window

```rust
use gpui_component::scroll::ScrollableElement;

pub fn open_floating_playlist(app: &mut App, player: Arc<Player>) {
    let _ = app.open_window(
        WindowOptions {
            titlebar: Some(TitlebarOptions { title: Some("播放列表".into()), ..Default::default() }),
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Point::default(),
                size: gpui::Size { width: px(420.0), height: px(720.0) },
            })),
            ..Default::default()
        },
        |window, cx| {
            let view = cx.new(|_cx| FloatingPlaylistView::new(player.clone()));
            cx.new(|cx| Root::new(view, window, cx))
        },
    );
}

impl Render for FloatingPlaylistView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tracks = self.player.playlist().tracks().to_vec();
        v_flex()
            .size_full()
            .bg(c.bg)
            .child(/* header */)
            .child(
                div()
                    .flex_grow()
                    .overflow_y_scrollbar()   // Scrollable<Div>; keep .py_1()/.children() after
                    .py_1()
                    .children(tracks.iter().enumerate().map(|(i, track)| {
                        Button::new(("fp_track", i))   // tuple id, NOT String
                            .w_full()
                            .on_click(move |_, _, _| { let _ = player.play_at_index(i); })
                            .child(/* label */)
                    })),
            )
    }
}
```

## Always-on-top (Windows only)

```rust
static ALWAYS_ON_TOP: AtomicBool = AtomicBool::new(false);

fn toggle_always_on_top() {
    let on = !ALWAYS_ON_TOP.fetch_xor(true, Ordering::Relaxed);
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            FindWindowW, SetWindowPos, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE,
        };
        use windows::Win32::Foundation::HWND;
        use windows::core::PCWSTR;
        let title: Vec<u16> = "HackMagic Music Player".encode_utf16()
            .chain(std::iter::once(0)).collect();
        if let Ok(hwnd) = unsafe { FindWindowW(None, PCWSTR::from_raw(title.as_ptr())) } {
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    Some(if on { HWND_TOPMOST } else { HWND::default() }), // Option<HWND>
                    0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE,
                );
            }
        }
    }
}
```

## Row click selection inside `.map()`

```rust
.children(rows.iter().enumerate().map(|(i, row)| {
    let weak = cx.entity().downgrade();
    h_flex()
        .on_mouse_down(gpui::MouseButton::Left,
            cx.listener(move |this: &mut MusicPlayer, _, _, _| {
                this.editor_state.selected_row = Some(i);
            }))
        .child(/* ... */)
}))
```
