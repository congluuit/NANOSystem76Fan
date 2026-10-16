//! System tray icon and menu.

use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::config::Mode;
use crate::i18n::Strings;
use crate::icon_art::render_icon;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    Show,
    SetMode(Mode),
    Quit,
}

pub struct Tray {
    icon: TrayIcon,
    show: MenuItem,
    auto: CheckMenuItem,
    fixed: CheckMenuItem,
    curve: CheckMenuItem,
    quit: MenuItem,
    last_tooltip: String,
    last_color: [u8; 3],
}

const ID_SHOW: &str = "show";
const ID_AUTO: &str = "auto";
const ID_FIXED: &str = "fixed";
const ID_CURVE: &str = "curve";
const ID_QUIT: &str = "quit";

fn make_icon(color: [u8; 3]) -> Option<Icon> {
    Icon::from_rgba(render_icon(32, color), 32, 32).ok()
}

impl Tray {
    /// Must be called on the UI thread (it owns the Win32 message loop).
    /// Tray events wake the UI through `ctx.request_repaint()` so they are
    /// processed even while the window is hidden.
    pub fn new(ctx: &egui::Context, s: &Strings, color: [u8; 3]) -> Option<(Self, Receiver<TrayAction>)> {
        let (tx, rx) = channel::<TrayAction>();

        let show = MenuItem::with_id(ID_SHOW, s.tray_show, true, None);
        let auto = CheckMenuItem::with_id(ID_AUTO, s.mode_auto, true, false, None);
        let fixed = CheckMenuItem::with_id(ID_FIXED, s.mode_fixed, true, false, None);
        let curve = CheckMenuItem::with_id(ID_CURVE, s.mode_curve, true, false, None);
        let quit = MenuItem::with_id(ID_QUIT, s.tray_quit, true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &show,
            &PredefinedMenuItem::separator(),
            &auto,
            &fixed,
            &curve,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .ok()?;

        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip("S76 Fan Control")
            .with_icon(make_icon(color)?)
            .build()
            .ok()?;

        let send = |tx: &Sender<TrayAction>, ctx: &egui::Context, a: TrayAction| {
            let _ = tx.send(a);
            ctx.request_repaint();
        };
        {
            let (tx, ctx) = (tx.clone(), ctx.clone());
            MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
                let action = match e.id.as_ref() {
                    ID_SHOW => TrayAction::Show,
                    ID_AUTO => TrayAction::SetMode(Mode::Auto),
                    ID_FIXED => TrayAction::SetMode(Mode::Fixed),
                    ID_CURVE => TrayAction::SetMode(Mode::Curve),
                    ID_QUIT => TrayAction::Quit,
                    _ => return,
                };
                send(&tx, &ctx, action);
            }));
        }
        {
            let ctx = ctx.clone();
            TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
                if let TrayIconEvent::DoubleClick { button: MouseButton::Left, .. }
                | TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: tray_icon::MouseButtonState::Up,
                    ..
                } = e
                {
                    send(&tx, &ctx, TrayAction::Show);
                }
            }));
        }

        Some((Self { icon, show, auto, fixed, curve, quit, last_tooltip: String::new(), last_color: color }, rx))
    }

    pub fn update(&mut self, s: &Strings, mode: Mode, tooltip: &str, color: [u8; 3]) {
        self.show.set_text(s.tray_show);
        self.quit.set_text(s.tray_quit);
        for (item, m, label) in [
            (&self.auto, Mode::Auto, s.mode_auto),
            (&self.fixed, Mode::Fixed, s.mode_fixed),
            (&self.curve, Mode::Curve, s.mode_curve),
        ] {
            item.set_text(label);
            item.set_checked(mode == m);
        }
        if tooltip != self.last_tooltip {
            let _ = self.icon.set_tooltip(Some(tooltip));
            self.last_tooltip = tooltip.to_string();
        }
        if color != self.last_color {
            let _ = self.icon.set_icon(make_icon(color));
            self.last_color = color;
        }
    }
}
