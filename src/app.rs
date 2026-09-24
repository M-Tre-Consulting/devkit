use std::cell::RefCell;
use std::rc::Rc;
use slint::ComponentHandle;
#[cfg(target_os = "android")]
use slint::Global;
use crate::navigation::{NavigationState, Screen};

pub fn run() -> Result<(), slint::PlatformError> {
    let ui = crate::AppWindow::new()?;
    setup_navigation(&ui);
    ui.run()
}

pub fn setup_navigation(ui: &crate::AppWindow) {
    let nav_state = Rc::new(RefCell::new(NavigationState::new()));

    let state_clone = nav_state.clone();
    let ui_handle = ui.as_weak();
    ui.on_navigate(move |screen_id| {
        let screen = Screen::from_id(screen_id);
        state_clone.borrow_mut().navigate_to(screen);
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_current_screen(screen.id());
        }
    });

    let state_clone = nav_state.clone();
    let ui_handle = ui.as_weak();
    ui.on_back(move || {
        let screen = state_clone.borrow_mut().navigate_back();
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_current_screen(screen.id());
        }
    });
}

#[cfg(target_os = "android")]
pub fn android_main(android_app: slint::android::AndroidApp) {
    slint::android::init(android_app).unwrap();
    let ui = crate::AppWindow::new().unwrap();
    crate::MaterialWindowAdapter::get(&ui).set_disable_hover(true);
    setup_navigation(&ui);
    ui.run().unwrap();
}
