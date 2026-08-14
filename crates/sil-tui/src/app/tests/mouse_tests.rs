use super::super::{ActiveTab, App, InputMode};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

#[test]
fn mouse_click_selects_tab() {
    let mut app = App::new(None);
    app.input_mode = InputMode::Normal;

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 17,
            row: 1,
            modifiers: crossterm::event::KeyModifiers::empty(),
        },
        80,
        24,
    );

    assert_eq!(app.active_tab, ActiveTab::Sources);
}

#[test]
fn mouse_ignores_non_left_clicks_and_modals() {
    let mut app = App::new(None);
    app.input_mode = InputMode::Normal;
    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 14,
            row: 1,
            modifiers: crossterm::event::KeyModifiers::empty(),
        },
        80,
        24,
    );
    assert_eq!(app.active_tab, ActiveTab::Dashboard);

    app.input_mode = InputMode::CommandPalette;
    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 14,
            row: 1,
            modifiers: crossterm::event::KeyModifiers::empty(),
        },
        80,
        24,
    );
    assert_eq!(app.active_tab, ActiveTab::Dashboard);
}
