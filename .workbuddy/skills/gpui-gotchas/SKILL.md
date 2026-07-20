---
name: gpui-gotchas
description: >-
  GPUI 0.2.2 + gpui-component 0.5.1 API gotchas for the HackMagic Music Player
  (Rust+GPUI) project in src/gui/mod.rs. This skill should be used when editing
  the GPUI UI in this workspace — wiring buttons/menus, adding scrollable
  panels, opening child windows, or calling Windows platform APIs — to avoid
  the non-obvious compile errors (Div scrolling, cx.listener arity, Root::new
  signature, Button id types, SetWindowPos Option<HWND>).
agent_created: true
---

# GPUI Gotchas — HackMagic Music Player

Reusable, non-obvious API facts for the GPUI UI layer of this project
(`src/gui/mod.rs` and siblings), verified against the pinned crates
`gpui 0.2.2` and `gpui-component 0.5.1`. Apply these whenever editing GUI code
or the build will fail on surprising errors.

## When to use

- Adding scrollable lists/panels (`overflow_y_scroll` not working on `Div`).
- Wiring `on_click` / `on_mouse_down` handlers via `cx.listener`.
- Opening a second/top-level window (`open_window` + `Root::new`).
- Creating `Button::new(...)` with a dynamic id.
- Calling `windows` 0.61 APIs (`FindWindowW`, `SetWindowPos`) for always-on-top.
- Drawing 1px dividers / borders.

## Core gotchas

1. **`Div` does NOT implement `StatefulInteractiveElement`** (only `Img` does).
   `div().overflow_y_scroll()` does not exist. To make a `Div` scrollable, import
   `gpui_component::scroll::ScrollableElement` and call `.overflow_y_scrollbar()`
   (or `overflow_x_scrollbar` / `overflow_scrollbar`). It returns
   `Scrollable<Div>`, which still supports `.py_1()` / `.children()` / `.bg()`
   chaining (Scrollable implements `Styled` + `ParentElement` + `IntoElement`).

2. **`cx.listener` closures are ALWAYS 4 args: `(entity, event, window, cx)`**
   i.e. `Fn(&mut T, &E, &mut Window, &mut Context<T>)`.
   - The entity (`this: &mut Self`) is the **1st** parameter.
   - The event (`&E`, e.g. `MouseButton` for `on_mouse_down`, `ClickEvent` for
     `on_click`) is the **2nd** parameter — NOT `window`.
   - Wrong (prior assumption): `cx.listener(|this, window, cx| ...)` for
     `on_click`. Correct: `cx.listener(|this, _, _, _| ...)` or, when you need the
     event, `cx.listener(move |this: &mut MusicPlayer, event, _, _| ...)`.
   - A plain 3-arg form (`|this, window, cx|`) causes `E0631` (closure arg
     mismatch). When the entity type cannot be inferred, annotate the 1st param:
     `|this: &mut MusicPlayer, _, _, _|`.

3. **`Root::new(view, window, cx)` needs `cx: &mut Context<Root>`, not `&mut App`.**
   Inside `app.open_window(WindowOptions{...}, |window, cx| { ... })` (where `cx`
   is `&mut App`), wrap the root creation:
   ```rust
   |window, cx| {
       let view = cx.new(|_cx| FloatingPlaylistView::new(player.clone()));
       cx.new(|cx| Root::new(view, window, cx))
   }
   ```
   `player` (an `Arc`) is captured by the outer closure by reference and cloned
   inside `cx.new`; no `move` needed on the outer closure.

4. **`Button::new(id)` takes `impl Into<ElementId>`; `String` does NOT implement
   `Into<ElementId>`.** Use a tuple `Button::new(("fp_track", i))` (the
   `(&'static str, usize)` impl exists) or `gpui::SharedString::from(...)`. A
   `format!(...).as_str()` borrows a temporary and won't live long enough.

5. **`windows` 0.61 `SetWindowPos` 2nd arg (`hWndInsertAfter`) is `Option<HWND>`.**
   Write `Some(HWND_TOPMOST)` / `Some(HWND::default())`, not bare `HWND_TOPMOST`.

6. **No `border_right_1` / `border_left_1`** in gpui 0.2.2. Draw 1px dividers as
   `div().w(px(1.0)).h_full().bg(c.border)` placed between columns.

7. **Don't capture `&mut self` inside a `move` closure that escapes the method**
   (e.g. a `menu_dropdown` builder). It triggers `E0521` "borrowed data escapes".
   Clone the needed field *before* the closure (`let pl = self.player.clone();`)
   and use `pl.clone()` inside the closure instead.

8. **`cx.listener` cannot be used inside a `.map()` closure** over rows (the
   borrowed `self` would escape). Use `let weak = cx.entity().downgrade();` then
   `weak.update(cx, |this, cx| { ... })` inside the `.map()` body.

9. **`IconName` is not `Copy`.** When passing it through a closure, use
   `.icon(icon.clone())`, not `.icon(*icon)`.

## Build & run

- Compile the GUI binary: `cargo build --bin hm-gui`.
- Run (requires a Windows display — cannot run headless): `cargo run --bin hm-gui`.
- `start.bat` is broken (references a missing `gui/` dir); do not use it.
- The GUI runs `MusicPlayer` on an 8 MB-stack thread in `src/bin/gui.rs` to avoid
  stack overflow.

## Reference

See `references/gpui_api.md` for a fuller worked example of a scrollable floating
playlist window and the always-on-top Windows call.
