use std::cell::RefCell;
use std::rc::Rc;
use slint::{ComponentHandle, ModelRc, VecModel};
#[cfg(target_os = "android")]
use slint::Global;
use crate::navigation::{AppMode, NavigationState};

pub fn run() -> Result<(), slint::PlatformError> {
    let ui = crate::AppWindow::new()?;
    let status_inset = crate::platform::android::get_status_bar_inset();
    let nav_inset = crate::platform::android::get_navigation_bar_inset();
    ui.set_status_bar_inset(status_inset);
    ui.set_nav_bar_inset(nav_inset);
    setup_app_state(&ui);
    ui.run()
}

pub fn update_favorites(ui: &crate::AppWindow) {
    let favs = crate::storage::load_favorites();
    let model = ModelRc::new(VecModel::from(vec![
        false,
        favs.contains(&1),
        favs.contains(&2),
        favs.contains(&3),
        favs.contains(&4),
        favs.contains(&5),
        favs.contains(&6),
    ]));
    ui.set_tool_favorites(model);
}

pub fn update_recents(ui: &crate::AppWindow) {
    let recents = crate::storage::load_recents();
    let model = ModelRc::new(VecModel::from(recents));
    ui.set_recent_tools(model);
}

pub fn setup_app_state(ui: &crate::AppWindow) {
    update_favorites(ui);
    update_recents(ui);

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
        let _ = crate::storage::record_tool_opened(tool_id);
        state_clone.borrow_mut().open_tool(tool_id);
        if let Some(ui) = ui_handle.upgrade() {
            update_recents(&ui);
            ui.set_active_tool(tool_id);
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_toggle_favorite(move |tool_id| {
        let _ = crate::storage::toggle_favorite(tool_id);
        if let Some(ui) = ui_handle.upgrade() {
            update_favorites(&ui);
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_clear_recents(move || {
        crate::storage::clear_recents();
        if let Some(ui) = ui_handle.upgrade() {
            update_recents(&ui);
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_clear_favorites(move || {
        crate::storage::clear_favorites();
        if let Some(ui) = ui_handle.upgrade() {
            update_favorites(&ui);
        }
    });

    let state_clone = nav_state.clone();
    let ui_handle = ui.as_weak();
    ui.on_back(move || {
        let outcome = state_clone.borrow_mut().handle_back();
        if let Some(ui) = ui_handle.upgrade() {
            match outcome {
                crate::navigation::BackNavigationOutcome::DismissModal => {
                    ui.set_modal_open(false);
                }
                crate::navigation::BackNavigationOutcome::CloseTool { return_to } => {
                    ui.set_active_tool(0);
                    ui.set_current_mode(return_to.id());
                }
                crate::navigation::BackNavigationOutcome::NavigateToHome => {
                    ui.set_active_tool(0);
                    ui.set_current_mode(crate::navigation::AppMode::Home.id());
                }
                crate::navigation::BackNavigationOutcome::ExitApp => {}
            }
        }
    });

    let state_clone = nav_state.clone();
    let ui_handle = ui.as_weak();
    ui.on_handle_back(move || -> bool {
        let outcome = state_clone.borrow_mut().handle_back();
        if let Some(ui) = ui_handle.upgrade() {
            match outcome {
                crate::navigation::BackNavigationOutcome::DismissModal => {
                    ui.set_modal_open(false);
                    true
                }
                crate::navigation::BackNavigationOutcome::CloseTool { return_to } => {
                    ui.set_active_tool(0);
                    ui.set_current_mode(return_to.id());
                    true
                }
                crate::navigation::BackNavigationOutcome::NavigateToHome => {
                    ui.set_active_tool(0);
                    ui.set_current_mode(crate::navigation::AppMode::Home.id());
                    true
                }
                crate::navigation::BackNavigationOutcome::ExitApp => false,
            }
        } else {
            false
        }
    });
}

#[cfg(target_os = "android")]
pub fn android_main(android_app: slint::android::AndroidApp) {
    if let Some(path) = android_app.internal_data_path() {
        crate::storage::init_storage_dir(path.to_path_buf());
    }
    let status_inset = crate::platform::android::query_status_bar_inset(&android_app);
    let nav_inset = crate::platform::android::query_navigation_bar_inset(&android_app);
    crate::platform::android::set_status_bar_inset(status_inset);
    crate::platform::android::set_navigation_bar_inset(nav_inset);
    slint::android::init(android_app).unwrap();
    let ui = crate::AppWindow::new().unwrap();
    ui.set_status_bar_inset(status_inset);
    ui.set_nav_bar_inset(nav_inset);
    crate::MaterialWindowAdapter::get(&ui).set_disable_hover(true);
    setup_app_state(&ui);
    ui.run().unwrap();
}
