use std::rc::Rc;

use gtk::{EventControllerKey, gdk, glib, prelude::*};

use crate::{
    app::zoom::ZoomState,
    editor::tabs::TabController,
};

pub fn attach(window: &impl IsA<gtk::Widget>, tabs: Rc<TabController>, zoom: Rc<ZoomState>) {
    let keys = EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);

    keys.connect_key_pressed(move |_, key, _, mods| {
        let ctrl  = mods.contains(gdk::ModifierType::CONTROL_MASK);
        let shift = mods.contains(gdk::ModifierType::SHIFT_MASK);

        if !ctrl {
            return glib::Propagation::Proceed;
        }

        match (shift, key) {
            // tab cycling — both browser conventions supported
            (false, gdk::Key::Tab)
            | (false, gdk::Key::ISO_Left_Tab)
            | (false, gdk::Key::Page_Down) => {
                tabs.cycle_next();
                glib::Propagation::Stop
            }
            (true, gdk::Key::Tab)
            | (true, gdk::Key::ISO_Left_Tab)
            | (false, gdk::Key::Page_Up) => {
                tabs.cycle_prev();
                glib::Propagation::Stop
            }

            (true, gdk::Key::t) | (true, gdk::Key::T) => {
                tabs.reopen_last_closed();
                glib::Propagation::Stop
            }
            (false, gdk::Key::w) | (false, gdk::Key::W) => {
                tabs.close_active();
                glib::Propagation::Stop
            }
            (false, gdk::Key::t) | (false, gdk::Key::T) => {
                tabs.open_new();
                glib::Propagation::Stop
            }
            (false, gdk::Key::r) | (false, gdk::Key::R) => {
                tabs.rename_active();
                glib::Propagation::Stop
            }
            (false, gdk::Key::f) | (false, gdk::Key::F) => {
                tabs.toggle_search();
                glib::Propagation::Stop
            }
            // open a foreign file (txt, md, rs, …) as a new tab
            (false, gdk::Key::o) | (false, gdk::Key::O) => {
                tabs.open_file_dialog();
                glib::Propagation::Stop
            }

            // jump to tab by position — Ctrl+1..9
            (false, gdk::Key::_1) => { tabs.jump_to(0); glib::Propagation::Stop }
            (false, gdk::Key::_2) => { tabs.jump_to(1); glib::Propagation::Stop }
            (false, gdk::Key::_3) => { tabs.jump_to(2); glib::Propagation::Stop }
            (false, gdk::Key::_4) => { tabs.jump_to(3); glib::Propagation::Stop }
            (false, gdk::Key::_5) => { tabs.jump_to(4); glib::Propagation::Stop }
            (false, gdk::Key::_6) => { tabs.jump_to(5); glib::Propagation::Stop }
            (false, gdk::Key::_7) => { tabs.jump_to(6); glib::Propagation::Stop }
            (false, gdk::Key::_8) => { tabs.jump_to(7); glib::Propagation::Stop }
            // Ctrl+9 jumps to last tab regardless of count, like Firefox/Chrome
            (false, gdk::Key::_9) => {
                let last = tabs.workspace.borrow().open_tabs().len().saturating_sub(1);
                tabs.jump_to(last);
                glib::Propagation::Stop
            }

            (false, gdk::Key::equal)
            | (false, gdk::Key::plus)
            | (false, gdk::Key::KP_Add) => {
                zoom.increase();
                glib::Propagation::Stop
            }
            (false, gdk::Key::minus) | (false, gdk::Key::KP_Subtract) => {
                zoom.decrease();
                glib::Propagation::Stop
            }
            (false, gdk::Key::_0) | (false, gdk::Key::KP_0) => {
                zoom.reset();
                glib::Propagation::Stop
            }

            _ => glib::Propagation::Proceed,
        }
    });

    window.add_controller(keys);
}
