// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 MTRE Consulting

use std::cell::RefCell;
use std::rc::Rc;
use slint::{ComponentHandle, ModelRc, VecModel};
#[cfg(target_os = "android")]
use slint::Global;
use crate::navigation::{AppMode, NavigationState};

pub fn setup_keyboard_observer(ui: &crate::AppWindow) -> slint::Timer {
    let kb_timer = slint::Timer::default();
    let ui_weak = ui.as_weak();
    let mut last_kb: f32 = 0.0;
    kb_timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(32), move || {
        let kb = crate::platform::android::query_keyboard_inset();
        if (kb - last_kb).abs() > 0.5 {
            last_kb = kb;
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_keyboard_height(kb);
                ui.set_keyboard_open(kb > 20.0);
            }
        }
    });
    kb_timer
}

pub fn run() -> Result<(), slint::PlatformError> {
    let ui = crate::AppWindow::new()?;
    let status_inset = crate::platform::android::get_status_bar_inset();
    let nav_inset = crate::platform::android::get_navigation_bar_inset();
    ui.set_status_bar_inset(status_inset);
    ui.set_nav_bar_inset(nav_inset);
    setup_app_state(&ui);
    let _kb_timer = setup_keyboard_observer(&ui);
    let high_rr = crate::storage::load_high_refresh_rate();
    let _ = std::panic::catch_unwind(|| {
        crate::platform::android::apply_refresh_rate_setting(high_rr);
    });
    ui.run()
}

pub fn update_favorites(ui: &crate::AppWindow) {
    let favs = crate::storage::load_favorites();
    let items: Vec<bool> = (0..=14).map(|i| i > 0 && favs.contains(&i)).collect();
    let model = ModelRc::new(VecModel::from(items));
    ui.set_tool_favorites(model);
}

pub fn update_recents(ui: &crate::AppWindow) {
    let recents = crate::storage::load_recents();
    let model = ModelRc::new(VecModel::from(recents));
    ui.set_recent_tools(model);
}

pub fn update_high_refresh_rate(ui: &crate::AppWindow) {
    let supported = crate::platform::android::is_high_refresh_rate_supported();
    ui.set_high_refresh_rate_supported(supported);
    let enabled = crate::storage::load_high_refresh_rate();
    ui.set_high_refresh_rate_enabled(enabled);
}

pub fn setup_tool_handlers(ui: &crate::AppWindow) {
    // 1. Subnet Calculator
    ui.on_subnet_calculate(move |ip| {
        crate::platform::android::haptic_tap();
        match crate::tools::handlers::handle_subnet(&ip) {
            Ok(out) => crate::SubnetResult {
                network_address: out.network_address.into(),
                broadcast_address: out.broadcast_address.into(),
                first_host: out.first_host.into(),
                last_host: out.last_host.into(),
                host_count: out.host_count.into(),
                netmask: out.netmask.into(),
                wildcard_mask: out.wildcard_mask.into(),
                cidr_notation: out.cidr_notation.into(),
                error_message: "".into(),
            },
            Err(err) => crate::SubnetResult {
                network_address: "".into(),
                broadcast_address: "".into(),
                first_host: "".into(),
                last_host: "".into(),
                host_count: "".into(),
                netmask: "".into(),
                wildcard_mask: "".into(),
                cidr_notation: "".into(),
                error_message: err.into(),
            },
        }
    });

    // 2. Hash Calculator
    ui.on_hash_calculate(move |text, algo| {
        crate::platform::android::haptic_tap();
        match crate::tools::handlers::handle_hash(&text, algo) {
            Ok(output) => crate::HashResult {
                hex_digest: output.hex_digest.into(),
                base64_digest: output.base64_digest.into(),
                error_message: "".into(),
            },
            Err(err) => crate::HashResult {
                hex_digest: "".into(),
                base64_digest: "".into(),
                error_message: err.into(),
            },
        }
    });

    // 3. Base64
    ui.on_base64_convert(move |text, is_enc, is_url, pad| {
        crate::platform::android::haptic_tap();
        match crate::tools::handlers::handle_base64(&text, is_enc, is_url, pad) {
            Ok(output) => crate::Base64Result {
                output_text: output.output_text.into(),
                error_message: "".into(),
            },
            Err(err) => crate::Base64Result {
                output_text: "".into(),
                error_message: err.into(),
            },
        }
    });

    // 4. UUID Generator
    ui.on_uuid_generate(move |ver, count, upper, hyphens| {
        crate::platform::android::haptic_tap();
        match crate::tools::handlers::handle_uuid(ver, count as usize, upper, hyphens) {
            Ok(out) => {
                let bulk = out.uuids.join("\n");
                let shared_uuids: Vec<slint::SharedString> =
                    out.uuids.into_iter().map(slint::SharedString::from).collect();
                crate::UuidResult {
                    uuids: ModelRc::new(VecModel::from(shared_uuids)),
                    bulk_uuids: bulk.into(),
                    error_message: "".into(),
                }
            }
            Err(err) => crate::UuidResult {
                uuids: ModelRc::new(VecModel::from(Vec::new())),
                bulk_uuids: "".into(),
                error_message: err.into(),
            },
        }
    });

    // 5. Timestamp Converter
    ui.on_timestamp_convert(move |ts, tz| {
        crate::platform::android::haptic_tap();
        match crate::tools::handlers::handle_timestamp(&ts, &tz) {
            Ok(out) => crate::TimestampResult {
                unix_seconds: out.unix_seconds.into(),
                unix_millis: out.unix_millis.into(),
                iso_8601: out.iso_8601.into(),
                rfc_2822: out.rfc_2822.into(),
                human_readable: out.human_readable.into(),
                error_message: "".into(),
            },
            Err(err) => crate::TimestampResult {
                unix_seconds: "".into(),
                unix_millis: "".into(),
                iso_8601: "".into(),
                rfc_2822: "".into(),
                human_readable: "".into(),
                error_message: err.into(),
            },
        }
    });

    // 6. Regex Tester
    ui.on_regex_test(move |pat, text, ci, ml, dot| {
        crate::platform::android::haptic_tap();
        match crate::tools::handlers::handle_regex(&pat, &text, ci, ml, dot) {
            Ok(out) => {
                let summary = if out.total_matches == 1 {
                    "1 match found".to_string()
                } else {
                    format!("{} matches found", out.total_matches)
                };
                let matches_formatted: Vec<slint::SharedString> = out
                    .matches
                    .into_iter()
                    .map(|m| format!("Match {}: '{}' [{}..{}]", m.index, m.text, m.start, m.end).into())
                    .collect();
                crate::RegexResult {
                    match_summary: summary.into(),
                    matches_list: ModelRc::new(VecModel::from(matches_formatted)),
                    error_message: "".into(),
                }
            }
            Err(err) => crate::RegexResult {
                match_summary: "".into(),
                matches_list: ModelRc::new(VecModel::from(Vec::new())),
                error_message: err.into(),
            },
        }
    });

    // 7. JSON ↔ YAML Converter
    ui.on_json_yaml_convert(move |src, is_j2y, ind| {
        crate::platform::android::haptic_tap();
        match crate::tools::handlers::handle_json_yaml(&src, is_j2y, ind as usize) {
            Ok(out) => crate::JsonYamlResult {
                converted_text: out.converted_text.into(),
                error_message: "".into(),
            },
            Err(err) => crate::JsonYamlResult {
                converted_text: "".into(),
                error_message: err.into(),
            },
        }
    });

    // 8. CRON Parser
    ui.on_cron_parse(move |expr| {
        crate::platform::android::haptic_tap();
        match crate::tools::handlers::handle_cron(&expr) {
            Ok(out) => {
                let runs: Vec<slint::SharedString> = out
                    .next_run_times
                    .into_iter()
                    .map(slint::SharedString::from)
                    .collect();
                crate::CronResult {
                    description: out.description.into(),
                    next_runs: ModelRc::new(VecModel::from(runs)),
                    error_message: "".into(),
                }
            }
            Err(err) => crate::CronResult {
                description: "".into(),
                next_runs: ModelRc::new(VecModel::from(Vec::new())),
                error_message: err.into(),
            },
        }
    });

    // 9. GZip Compressor
    ui.on_gzip_compress(move |data, is_comp| {
        crate::platform::android::haptic_tap();
        match crate::tools::handlers::handle_gzip(&data, is_comp) {
            Ok(out) => crate::GzipResult {
                result_data: out.result_data.into(),
                original_size: format!("{} bytes", out.original_size).into(),
                processed_size: format!("{} bytes", out.processed_size).into(),
                ratio: format!("{:.1}% reduction", out.ratio_percentage).into(),
                error_message: "".into(),
            },
            Err(err) => crate::GzipResult {
                result_data: "".into(),
                original_size: "".into(),
                processed_size: "".into(),
                ratio: "".into(),
                error_message: err.into(),
            },
        }
    });

    // 10. Code Formatter
    ui.on_format_format(move |src, lang, ind| {
        crate::platform::android::haptic_tap();
        match crate::tools::handlers::handle_formatter(&src, lang, ind as u8) {
            Ok(out) => crate::FormatterResult {
                formatted_code: out.formatted_code.into(),
                error_message: "".into(),
            },
            Err(err) => crate::FormatterResult {
                formatted_code: "".into(),
                error_message: err.into(),
            },
        }
    });

    // 11. Chmod Calculator
    ui.on_chmod_apply(move |or, ow, ox, gr, gw, gx, tr, tw, tx| {
        crate::platform::android::haptic_tap();
        match crate::tools::handlers::handle_chmod(or, ow, ox, gr, gw, gx, tr, tw, tx) {
            Ok(out) => crate::ChmodResult {
                octal_output: out.octal.into(),
                symbolic_output: out.symbolic.into(),
                command_output: out.command.into(),
                error_message: "".into(),
            },
            Err(err) => crate::ChmodResult {
                octal_output: "".into(),
                symbolic_output: "".into(),
                command_output: "".into(),
                error_message: err.into(),
            },
        }
    });

    // 12. Color Converter
    ui.on_color_convert(move |val, fmt| {
        crate::platform::android::haptic_tap();
        match crate::tools::handlers::handle_color(&val, fmt) {
            Ok(out) => crate::ColorResult {
                hex_output: out.hex.into(),
                rgb_output: out.rgb.into(),
                hsl_output: out.hsl.into(),
                hsv_output: out.hsv.into(),
                cmyk_output: out.cmyk.into(),
                error_message: "".into(),
            },
            Err(err) => crate::ColorResult {
                hex_output: "".into(),
                rgb_output: "".into(),
                hsl_output: "".into(),
                hsv_output: "".into(),
                cmyk_output: "".into(),
                error_message: err.into(),
            },
        }
    });

    // 13. Contrast Checker
    ui.on_contrast_check(move |fg, bg| {
        crate::platform::android::haptic_tap();
        match crate::tools::handlers::handle_contrast(&fg, &bg) {
            Ok(out) => crate::ContrastCheckResult {
                ratio_text: out.ratio_display.into(),
                aa_normal_pass: out.aa_normal,
                aa_large_pass: out.aa_large,
                aaa_normal_pass: out.aaa_normal,
                aaa_large_pass: out.aaa_large,
                ui_components_pass: out.ui_components,
                error_message: "".into(),
            },
            Err(err) => crate::ContrastCheckResult {
                ratio_text: "".into(),
                aa_normal_pass: false,
                aa_large_pass: false,
                aaa_normal_pass: false,
                aaa_large_pass: false,
                ui_components_pass: false,
                error_message: err.into(),
            },
        }
    });

    // 14. JWT Decoder
    ui.on_jwt_decode(move |token| {
        crate::platform::android::haptic_tap();
        match crate::tools::handlers::handle_jwt(&token) {
            Ok(out) => crate::JwtResult {
                header_json: out.header_json.into(),
                payload_json: out.payload_json.into(),
                signature_output: out.signature.into(),
                algorithm: out.algorithm.into(),
                expiration_status: out.expiration_status.as_str().into(),
                expiration_time: out.expiration_time.into(),
                error_message: "".into(),
            },
            Err(err) => crate::JwtResult {
                header_json: "".into(),
                payload_json: "".into(),
                signature_output: "".into(),
                algorithm: "".into(),
                expiration_status: "".into(),
                expiration_time: "".into(),
                error_message: err.into(),
            },
        }
    });

    // Clipboard copy action
    ui.on_copy_text(move |_text| {
        crate::platform::android::haptic_tap();
        // Ready for system clipboard integration
    });
}

pub fn setup_app_state(ui: &crate::AppWindow) {
    update_favorites(ui);
    update_recents(ui);
    update_high_refresh_rate(ui);
    setup_tool_handlers(ui);

    let nav_state = Rc::new(RefCell::new(NavigationState::new()));

    let state_clone = nav_state.clone();
    let ui_handle = ui.as_weak();
    ui.on_switch_mode(move |mode_id| {
        crate::platform::android::haptic_tap();
        let mode = AppMode::from_id(mode_id);
        state_clone.borrow_mut().switch_mode(mode);
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_current_mode(mode.id());
            ui.set_active_tool(0);
            ui.set_about_open(false);
            ui.set_modal_open(false);
        }
    });

    let state_clone = nav_state.clone();
    let ui_handle = ui.as_weak();
    ui.on_open_tool(move |tool_id| {
        crate::platform::android::haptic_tap();
        let _ = crate::storage::record_tool_opened(tool_id);
        state_clone.borrow_mut().open_tool(tool_id);
        if let Some(ui) = ui_handle.upgrade() {
            update_recents(&ui);
            ui.set_active_tool(tool_id);
            ui.set_about_open(false);
            ui.set_modal_open(false);
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_toggle_favorite(move |tool_id| {
        crate::platform::android::haptic_tap();
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

    let ui_handle = ui.as_weak();
    ui.on_high_refresh_rate_changed(move |enabled| {
        crate::platform::android::haptic_tap();
        crate::storage::save_high_refresh_rate(enabled);
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_high_refresh_rate_enabled(enabled);
        }
        let _ = std::panic::catch_unwind(|| {
            crate::platform::android::apply_refresh_rate_setting(enabled);
        });
    });

    let state_clone = nav_state.clone();
    let ui_handle = ui.as_weak();
    ui.on_open_about(move || {
        state_clone.borrow_mut().open_modal();
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_about_open(true);
            ui.set_modal_open(true);
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
                    ui.set_about_open(false);
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
                    ui.set_about_open(false);
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
    crate::platform::android::configure_window_soft_input_mode(&android_app);
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
    let high_rr = crate::storage::load_high_refresh_rate();
    let _ = std::panic::catch_unwind(|| {
        crate::platform::android::apply_refresh_rate_setting(high_rr);
    });
    let _kb_timer = setup_keyboard_observer(&ui);
    ui.run().unwrap();
}
