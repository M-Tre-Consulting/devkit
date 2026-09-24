use std::cell::RefCell;
use std::rc::Rc;
use slint::ComponentHandle;
#[cfg(target_os = "android")]
use slint::Global;
use crate::navigation::{AppMode, NavigationState};

pub fn run() -> Result<(), slint::PlatformError> {
    let ui = crate::AppWindow::new()?;
    let inset = crate::platform::android::get_status_bar_inset();
    ui.set_status_bar_inset(inset);
    setup_navigation(&ui);
    ui.run()
}

pub fn setup_navigation(ui: &crate::AppWindow) {
    let nav_state = Rc::new(RefCell::new(NavigationState::new()));

    let state_clone = nav_state.clone();
    let ui_handle = ui.as_weak();
    ui.on_switch_mode(move |mode_id| {
        let mode = AppMode::from_id(mode_id);
        state_clone.borrow_mut().switch_mode(mode);
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_current_mode(mode.id());
            ui.set_active_tool(0);
        }
    });

    let state_clone = nav_state.clone();
    let ui_handle = ui.as_weak();
    ui.on_open_tool(move |tool_id| {
        state_clone.borrow_mut().open_tool(tool_id);
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_active_tool(tool_id);
        }
    });

    let state_clone = nav_state.clone();
    let ui_handle = ui.as_weak();
    ui.on_back(move || {
        let mode = state_clone.borrow_mut().close_tool();
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_active_tool(0);
            ui.set_current_mode(mode.id());
        }
    });
}

#[cfg(target_os = "android")]
pub fn android_main(android_app: slint::android::AndroidApp) {
    let inset = crate::platform::android::query_status_bar_inset(&android_app);
    crate::platform::android::set_status_bar_inset(inset);
    slint::android::init(android_app).unwrap();
    let ui = crate::AppWindow::new().unwrap();
    ui.set_status_bar_inset(inset);
    crate::MaterialWindowAdapter::get(&ui).set_disable_hover(true);
    setup_navigation(&ui);
    ui.run().unwrap();
}
