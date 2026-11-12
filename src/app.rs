// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, VecModel};
#[cfg(target_os = "android")]
use slint::Global;

use crate::catalog::{self, ToolDef};
use crate::navigation::{AppMode, BackNavigationOutcome, NavigationState};
use crate::tools::smart_detect::{self, Suggestion};

/// UI state owned by the Rust side and shared between callbacks.
///
/// The three tool lists are long-lived models updated *in place*: replacing the whole
/// model on every favorite toggle (as before) destroyed and rebuilt every card, which
/// dropped the tap that was still being delivered to the star and restarted animations.
struct AppState {
    nav: RefCell<NavigationState>,
    home: Rc<VecModel<crate::ToolInfo>>,
    favorites: Rc<VecModel<crate::ToolInfo>>,
    recents: Rc<VecModel<crate::ToolInfo>>,
    query: RefCell<String>,
    suggestions: RefCell<Vec<Suggestion>>,
    feedback_timer: slint::Timer,
}

impl AppState {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            nav: RefCell::new(NavigationState::new()),
            home: Rc::new(VecModel::default()),
            favorites: Rc::new(VecModel::default()),
            recents: Rc::new(VecModel::default()),
            query: RefCell::new(String::new()),
            suggestions: RefCell::new(Vec::new()),
            feedback_timer: slint::Timer::default(),
        })
    }

    /// Rebuild Home (filtered by the search query), Favorites and Recent from storage.
    fn refresh_lists(&self) {
        let favs = crate::storage::load_favorites();
        let recents = crate::storage::load_recents();
        let to_rows = |defs: Vec<&'static ToolDef>| -> Vec<crate::ToolInfo> {
            defs.into_iter().map(|t| tool_info(t, &favs)).collect()
        };
        let home = to_rows(catalog::search(&self.query.borrow()));
        let fav_rows = to_rows(favs.iter().filter_map(|id| catalog::find(*id)).collect());
        let recent_rows = to_rows(recents.iter().filter_map(|id| catalog::find(*id)).collect());
        sync_model(&self.home, home);
        sync_model(&self.favorites, fav_rows);
        sync_model(&self.recents, recent_rows);
    }
}

fn tool_info(t: &ToolDef, favorites: &[i32]) -> crate::ToolInfo {
    crate::ToolInfo {
        id: t.id,
        title: t.title.into(),
        description: t.description.into(),
        accent: t.accent,
        is_favorite: favorites.contains(&t.id),
    }
}

/// Bring `model` to `rows` with the smallest change: per-row updates when the ids are the
/// same, a single remove / insert when exactly one tool left or joined, a reset otherwise.
fn sync_model(model: &VecModel<crate::ToolInfo>, rows: Vec<crate::ToolInfo>) {
    let old: Vec<crate::ToolInfo> = model.iter().collect();
    let old_ids: Vec<i32> = old.iter().map(|t| t.id).collect();
    let new_ids: Vec<i32> = rows.iter().map(|t| t.id).collect();

    if old_ids == new_ids {
        for (i, row) in rows.into_iter().enumerate() {
            if old[i] != row {
                model.set_row_data(i, row);
            }
        }
        return;
    }
    if old_ids.len() == new_ids.len() + 1 {
        if let Some(pos) = (0..old_ids.len()).find(|&i| {
            let mut ids = old_ids.clone();
            ids.remove(i);
            ids == new_ids
        }) {
            model.remove(pos);
            return sync_model(model, rows);
        }
    }
    if new_ids.len() == old_ids.len() + 1 {
        if let Some(pos) = (0..new_ids.len()).find(|&i| {
            let mut ids = new_ids.clone();
            ids.remove(i);
            ids == old_ids
        }) {
            model.insert(pos, rows[pos].clone());
            return sync_model(model, rows);
        }
    }
    model.set_vec(rows);
}

/// Mode a tool should start in for a Smart Paste suggestion.
fn prefill_mode_for(s: &Suggestion) -> i32 {
    match s.tool_id {
        3 => 1,                                           // Base64: decode
        5 if !s.kind.starts_with("Unix") => 1,            // Timestamp: date -> epoch
        7 if s.kind.starts_with("YAML") => 1,             // YAML -> JSON
        10 if s.kind == "Markup" => 2,                    // Formatter: XML
        17 if s.kind == "cURL command" => 1,              // HTTP: import cURL
        _ => 0,
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn update_theme_settings(ui: &crate::AppWindow) {
    ui.set_dark_theme_enabled(crate::storage::load_dark_theme());
    ui.set_dynamic_color_enabled(crate::storage::load_dynamic_color());

    if let Some(c) = crate::platform::android::get_system_monet_colors() {
        let rgb = |v: (u8, u8, u8)| slint::Color::from_rgb_u8(v.0, v.1, v.2);
        ui.set_has_monet(true);
        ui.set_monet_primary_light(rgb(c.primary_light));
        ui.set_monet_primary_dark(rgb(c.primary_dark));
        ui.set_monet_primary_container_light(rgb(c.primary_container_light));
        ui.set_monet_primary_container_dark(rgb(c.primary_container_dark));
        ui.set_monet_secondary_light(rgb(c.secondary_light));
        ui.set_monet_secondary_dark(rgb(c.secondary_dark));
        ui.set_monet_tertiary_light(rgb(c.tertiary_light));
        ui.set_monet_tertiary_dark(rgb(c.tertiary_dark));
        ui.set_monet_surface_light(rgb(c.surface_light));
        ui.set_monet_surface_dark(rgb(c.surface_dark));
    }
}

fn update_tool_preferences(ui: &crate::AppWindow) {
    ui.set_default_hash_algorithm(crate::storage::load_default_hash_algorithm());
    ui.set_default_uuid_version(crate::storage::load_default_uuid_version());
    ui.set_default_timestamp_format(crate::storage::load_default_timestamp_format());
    ui.set_graphics_backend(crate::storage::load_graphics_backend());
    ui.set_peak_refresh_enabled(crate::storage::load_high_refresh_rate());
    ui.set_active_renderer(renderer_label().into());
}

/// Human readable description of the renderer chosen at startup.
fn renderer_label() -> String {
    if !cfg!(target_os = "android") {
        return "Desktop default".into();
    }
    let pref = crate::storage::load_graphics_backend();
    let api = crate::platform::android::resolve_graphics_api(pref);
    let mut label = api.label().to_string();
    if pref == 0 {
        label.push_str(" (auto");
        if crate::platform::android::is_tensor_g5_or_newer() {
            label.push_str(", Tensor G5+ detected");
        }
        label.push(')');
    }
    label
}

/// Desktop entry point.
pub fn run() -> Result<(), slint::PlatformError> {
    let ui = crate::AppWindow::new()?;
    ui.set_welcome_open(!crate::storage::load_has_seen_welcome());
    setup_app_state(&ui);
    // Development aid: DEVKIT_START_MODE=<0..3> / DEVKIT_START_TOOL=<id> open a screen directly.
    if let Some(mode) = std::env::var("DEVKIT_START_MODE").ok().and_then(|v| v.parse::<i32>().ok()) {
        ui.invoke_switch_mode(mode);
    }
    if let Some(id) = std::env::var("DEVKIT_START_TOOL").ok().and_then(|v| v.parse::<i32>().ok()) {
        ui.invoke_open_tool(id);
    }
    ui.run()
}

pub fn setup_tool_handlers(ui: &crate::AppWindow) {
    // 1. Subnet Calculator
    ui.on_subnet_calculate(move |ip| {
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
    ui.on_timestamp_convert(move |ts, mode, tz| {
        match crate::tools::handlers::handle_timestamp(&ts, mode, &tz) {
            Ok(out) => crate::TimestampResult {
                input_value: out.input_value.into(),
                unix_seconds: out.unix_seconds.into(),
                unix_millis: out.unix_millis.into(),
                iso_8601: out.iso_8601.into(),
                rfc_2822: out.rfc_2822.into(),
                human_readable: out.human_readable.into(),
                error_message: "".into(),
            },
            Err(err) => crate::TimestampResult {
                input_value: "".into(),
                unix_seconds: "".into(),
                unix_millis: "".into(),
                iso_8601: "".into(),
                rfc_2822: "".into(),
                human_readable: "".into(),
                error_message: err.into(),
            },
        }
    });

    ui.on_timestamp_now(move |mode, tz| {
        match crate::tools::handlers::handle_timestamp_now(mode, &tz) {
            Ok(out) => crate::TimestampResult {
                input_value: out.input_value.into(),
                unix_seconds: out.unix_seconds.into(),
                unix_millis: out.unix_millis.into(),
                iso_8601: out.iso_8601.into(),
                rfc_2822: out.rfc_2822.into(),
                human_readable: out.human_readable.into(),
                error_message: "".into(),
            },
            Err(err) => crate::TimestampResult {
                input_value: "".into(),
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
                    .map(|m| {
                        if m.captures.is_empty() {
                            format!("Match {}: '{}' [{}..{}]", m.index, m.text, m.start, m.end).into()
                        } else {
                            let groups = m
                                .captures
                                .iter()
                                .map(|g| format!("'{}'", g))
                                .collect::<Vec<_>>()
                                .join(", ");
                            format!(
                                "Match {}: '{}' [{}..{}] (groups: {})",
                                m.index, m.text, m.start, m.end, groups
                            )
                            .into()
                        }
                    })
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
        match crate::tools::handlers::handle_gzip(&data, is_comp) {
            Ok(out) => {
                let ratio_text = if out.original_size == 0 {
                    "—".to_string()
                } else {
                    let factor = out.processed_size as f32 / out.original_size as f32;
                    if out.processed_size < out.original_size {
                        let reduction = (1.0 - factor) * 100.0;
                        format!("{:.1}% reduction ({:.2}x)", reduction, factor)
                    } else if out.processed_size == out.original_size {
                        "0.0% change (1.00x)".to_string()
                    } else {
                        let increase = (factor - 1.0) * 100.0;
                        format!("{:.1}% increase ({:.2}x)", increase, factor)
                    }
                };
                crate::GzipResult {
                    result_data: out.result_data.into(),
                    original_size: format!("{} bytes", out.original_size).into(),
                    processed_size: format!("{} bytes", out.processed_size).into(),
                    ratio: ratio_text.into(),
                    error_message: "".into(),
                }
            }
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
        match crate::tools::handlers::handle_chmod(or, ow, ox, gr, gw, gx, tr, tw, tx) {
            Ok(out) => crate::ChmodResult {
                octal_output: out.octal.into(),
                symbolic_output: out.symbolic.into(),
                command_output: out.command.into(),
                error_message: "".into(),
                has_matrix: false,
                owner_r: or,
                owner_w: ow,
                owner_x: ox,
                group_r: gr,
                group_w: gw,
                group_x: gx,
                other_r: tr,
                other_w: tw,
                other_x: tx,
            },
            Err(err) => crate::ChmodResult {
                octal_output: "".into(),
                symbolic_output: "".into(),
                command_output: "".into(),
                error_message: err.into(),
                has_matrix: false,
                owner_r: false,
                owner_w: false,
                owner_x: false,
                group_r: false,
                group_w: false,
                group_x: false,
                other_r: false,
                other_w: false,
                other_x: false,
            },
        }
    });

    ui.on_chmod_apply_octal(move |raw_octal| {
        let octal_str = raw_octal.as_str();
        match crate::tools::handlers::handle_chmod_octal(octal_str) {
            Ok(out) => {
                let matrix = crate::tools::chmod::parse_octal(octal_str).ok();
                if let Some(crate::tools::chmod::ChmodInput::Matrix { owner, group, other }) = matrix {
                    crate::ChmodResult {
                        octal_output: out.octal.into(),
                        symbolic_output: out.symbolic.into(),
                        command_output: out.command.into(),
                        error_message: "".into(),
                        has_matrix: true,
                        owner_r: owner.read,
                        owner_w: owner.write,
                        owner_x: owner.execute,
                        group_r: group.read,
                        group_w: group.write,
                        group_x: group.execute,
                        other_r: other.read,
                        other_w: other.write,
                        other_x: other.execute,
                    }
                } else {
                    crate::ChmodResult {
                        octal_output: out.octal.into(),
                        symbolic_output: out.symbolic.into(),
                        command_output: out.command.into(),
                        error_message: "".into(),
                        has_matrix: false,
                        owner_r: false,
                        owner_w: false,
                        owner_x: false,
                        group_r: false,
                        group_w: false,
                        group_x: false,
                        other_r: false,
                        other_w: false,
                        other_x: false,
                    }
                }
            }
            Err(err) => crate::ChmodResult {
                octal_output: "".into(),
                symbolic_output: "".into(),
                command_output: "".into(),
                error_message: err.into(),
                has_matrix: false,
                owner_r: false,
                owner_w: false,
                owner_x: false,
                group_r: false,
                group_w: false,
                group_x: false,
                other_r: false,
                other_w: false,
                other_x: false,
            },
        }
    });

    // 12. Color Converter
    ui.on_color_convert(move |val, fmt| {
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

    // 15. Diff Studio
    ui.on_diff_compare(move |orig, modif, ignore_ws| {
        match crate::tools::handlers::handle_diff(&orig, &modif, ignore_ws) {
            Ok(out) => crate::DiffOutputResult {
                additions: out.additions as i32,
                deletions: out.deletions as i32,
                unchanged: out.unchanged as i32,
                unified_patch: out.unified_patch.into(),
                error_message: "".into(),
            },
            Err(err) => crate::DiffOutputResult {
                additions: 0,
                deletions: 0,
                unchanged: 0,
                unified_patch: "".into(),
                error_message: err.into(),
            },
        }
    });

    // 16. Developer QR Studio
    ui.on_qrcode_generate_text(move |text| {
        match crate::tools::handlers::handle_qrcode_text(&text) {
            Ok(qr) => {
                let img = crate::tools::handlers::qr_to_slint_image(&qr);
                crate::QrOutputResult {
                    qr_image: img,
                    payload: qr.payload.into(),
                    totp_code: "".into(),
                    totp_remaining_secs: 0,
                    error_message: "".into(),
                }
            }
            Err(err) => crate::QrOutputResult {
                qr_image: slint::Image::default(),
                payload: "".into(),
                totp_code: "".into(),
                totp_remaining_secs: 0,
                error_message: err.into(),
            },
        }
    });

    ui.on_qrcode_generate_wifi(move |ssid, pass, auth, hidden| {
        match crate::tools::handlers::handle_qrcode_wifi(&ssid, &pass, auth, hidden) {
            Ok(qr) => {
                let img = crate::tools::handlers::qr_to_slint_image(&qr);
                crate::QrOutputResult {
                    qr_image: img,
                    payload: qr.payload.into(),
                    totp_code: "".into(),
                    totp_remaining_secs: 0,
                    error_message: "".into(),
                }
            }
            Err(err) => crate::QrOutputResult {
                qr_image: slint::Image::default(),
                payload: "".into(),
                totp_code: "".into(),
                totp_remaining_secs: 0,
                error_message: err.into(),
            },
        }
    });

    ui.on_qrcode_generate_totp(move |issuer, account, secret| {
        match crate::tools::handlers::handle_qrcode_totp(&issuer, &account, &secret) {
            Ok(qr) => {
                let img = crate::tools::handlers::qr_to_slint_image(&qr);
                crate::QrOutputResult {
                    qr_image: img,
                    payload: qr.payload.into(),
                    totp_code: qr.totp_code.unwrap_or_default().into(),
                    totp_remaining_secs: qr.totp_remaining_secs.unwrap_or(0) as i32,
                    error_message: "".into(),
                }
            }
            Err(err) => crate::QrOutputResult {
                qr_image: slint::Image::default(),
                payload: "".into(),
                totp_code: "".into(),
                totp_remaining_secs: 0,
                error_message: err.into(),
            },
        }
    });


    // 16b. Live TOTP countdown (driven by a Timer in the UI while the QR Studio is open)
    ui.on_totp_tick(move |secret| match crate::tools::qrcode_tool::totp_now(&secret, now_secs()) {
        Ok((code, remaining)) => crate::TotpTick { code: code.into(), remaining: remaining as i32 },
        Err(_) => crate::TotpTick { code: "------".into(), remaining: 0 },
    });

    setup_network_handlers(ui);

    ui.on_http_import_curl(move |cmd| {
        match crate::tools::handlers::handle_http_import_curl(&cmd) {
            Ok(req) => crate::HttpImportResult {
                method_idx: crate::tools::handlers::http_method_to_int(req.method),
                url: req.url.into(),
                headers: crate::tools::http_client::format_headers_str(&req.headers).trim_end().into(),
                body: req.body.into(),
                error_message: "".into(),
            },
            Err(err) => crate::HttpImportResult {
                method_idx: 0,
                url: "".into(),
                headers: "".into(),
                body: "".into(),
                error_message: err.into(),
            },
        }
    });

    ui.on_http_export_curl(move |method, url, headers, body| {
        crate::tools::handlers::handle_http_export_curl(method, &url, &headers, &body).into()
    });

    ui.on_copy_text(move |text| {
        crate::platform::android::haptic_tap();
        let _ = crate::platform::android::copy_to_clipboard(text.as_str());
    });
}

/// HTTP, LAN Scout and TLS run blocking network I/O: each request runs on a worker
/// thread and posts its result back to the UI thread, so the UI never freezes (before,
/// a slow request or a port scan blocked rendering and input for seconds).
fn setup_network_handlers(ui: &crate::AppWindow) {
    // 17. HTTP Client
    let ui_weak = ui.as_weak();
    ui.on_http_send(move |method, url, headers, body| {
        let Some(ui) = ui_weak.upgrade() else { return };
        if ui.get_http_busy() {
            return;
        }
        crate::platform::android::haptic_tap();
        ui.set_http_busy(true);
        let (url, headers, body) = (url.to_string(), headers.to_string(), body.to_string());
        let weak = ui_weak.clone();
        std::thread::spawn(move || {
            let res = crate::tools::handlers::handle_http_send(method, &url, &headers, &body);
            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_http_result(http_output(res));
                ui.set_http_busy(false);
            });
        });
    });

    // 18. LAN Scout: port scan (mode 0 = known dev services, 1 = custom port list)
    let ui_weak = ui.as_weak();
    ui.on_lan_scout_scan(move |host, timeout_ms, mode, ports| {
        let Some(ui) = ui_weak.upgrade() else { return };
        if ui.get_lan_busy() {
            return;
        }
        crate::platform::android::haptic_tap();
        let timeout = if timeout_ms <= 0 { 600 } else { timeout_ms as u64 };
        let host = host.trim().to_string();
        let port_list: Result<Vec<u16>, String> = if mode == 1 {
            crate::tools::lan_scout::parse_port_spec(&ports)
        } else {
            Ok(crate::tools::lan_scout::KNOWN_SERVICES.iter().map(|s| s.port).collect())
        };
        let port_list = match port_list {
            Ok(p) => p,
            Err(e) => {
                ui.set_lan_result(lan_error(e));
                return;
            }
        };
        ui.set_lan_busy(true);
        ui.set_lan_progress(format!("Probing {} port(s) on {host}…", port_list.len()).into());
        let weak = ui_weak.clone();
        std::thread::spawn(move || {
            let res = std::panic::catch_unwind(|| crate::tools::lan_scout::scan_ports(&host, &port_list, timeout))
                .unwrap_or_else(|_| Err("Port scan failed unexpectedly".into()));
            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_lan_result(match res {
                    Ok(results) => lan_ports_output(results),
                    Err(e) => lan_error(e),
                });
                ui.set_lan_busy(false);
                ui.set_lan_progress("".into());
            });
        });
    });

    // 18. LAN Scout: host discovery on the local subnet
    let ui_weak = ui.as_weak();
    ui.on_lan_scout_discover(move |timeout_ms| {
        let Some(ui) = ui_weak.upgrade() else { return };
        if ui.get_lan_busy() {
            return;
        }
        crate::platform::android::haptic_tap();
        ui.set_lan_busy(true);
        ui.set_lan_progress("Reading network configuration…".into());
        let timeout = if timeout_ms <= 0 { 400 } else { timeout_ms as u64 };
        let weak = ui_weak.clone();
        std::thread::spawn(move || {
            let net = crate::tools::lan_scout::detect_network_identity();
            publish_network(&weak, &net);
            let device = net.device_ip.clone().unwrap_or_default();
            let gateway = net.gateway_ip.clone().unwrap_or_default();
            let prefix = net.prefix_len.unwrap_or(24);
            let probed = crate::tools::lan_scout::sweep_targets(&device, prefix).map(|t| t.len()).unwrap_or(0);
            if device.is_empty() || device.starts_with("127.") {
                let _ = weak.upgrade_in_event_loop(|ui| {
                    ui.set_lan_result(lan_error("Not connected to a local network (Wi-Fi or Ethernet)".into()));
                    ui.set_lan_busy(false);
                    ui.set_lan_progress("".into());
                });
                return;
            }
            let msg = format!("Sweeping {probed} addresses around {device}…");
            let _ = weak.upgrade_in_event_loop(move |ui| ui.set_lan_progress(msg.into()));
            let res = std::panic::catch_unwind(|| {
                crate::tools::lan_scout::discover_hosts(&device, prefix, &gateway, timeout)
            })
            .unwrap_or_else(|_| Err("Host discovery failed unexpectedly".into()));
            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_lan_result(match res {
                    Ok(hosts) => lan_hosts_output(hosts, probed),
                    Err(e) => lan_error(e),
                });
                ui.set_lan_busy(false);
                ui.set_lan_progress("".into());
            });
        });
    });

    let ui_weak = ui.as_weak();
    ui.on_refresh_network(move || {
        crate::platform::android::haptic_tap();
        refresh_network_async(ui_weak.clone());
    });

    // 19. TLS Inspector
    let ui_weak = ui.as_weak();
    ui.on_tls_inspect(move |target| {
        let Some(ui) = ui_weak.upgrade() else { return };
        if ui.get_tls_busy() {
            return;
        }
        crate::platform::android::haptic_tap();
        ui.set_tls_busy(true);
        let target = target.to_string();
        let weak = ui_weak.clone();
        std::thread::spawn(move || {
            let res = std::panic::catch_unwind(|| crate::tools::tls_inspector::inspect(&target, 8000))
                .unwrap_or_else(|_| Err("TLS inspection failed unexpectedly".into()));
            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_tls_result(tls_output(res));
                ui.set_tls_busy(false);
            });
        });
    });
}

fn http_output(res: Result<crate::tools::http_client::HttpResponse, String>) -> crate::HttpOutputResult {
    match res {
        Ok(r) => {
            let headers = r.headers.iter().map(|(k, v)| format!("{k}: {v}")).collect::<Vec<_>>().join("\n");
            let body = if r.pretty_body.is_empty() { r.body } else { r.pretty_body };
            crate::HttpOutputResult {
                status: r.status_code as i32,
                status_text: r.status_text.into(),
                latency_ms: r.latency_ms.min(i32::MAX as u64) as i32,
                ttfb_ms: r.ttfb_ms.min(i32::MAX as u64) as i32,
                size_bytes: r.content_length.min(i32::MAX as usize) as i32,
                headers: headers.into(),
                body: body.into(),
                final_url: r.final_url.into(),
                content_type: r.content_type.into(),
                truncated: r.truncated,
                error_message: "".into(),
            }
        }
        Err(err) => crate::HttpOutputResult {
            status_text: "Failed".into(),
            error_message: err.into(),
            ..Default::default()
        },
    }
}

fn port_item(r: &crate::tools::lan_scout::PortScanResult) -> crate::PortItem {
    crate::PortItem {
        port: r.port as i32,
        is_open: r.is_open,
        service_name: r.service_name.as_str().into(),
        service_category: r.service_category.as_str().into(),
        description: r.description.as_str().into(),
        latency_ms: r.latency_ms.min(i32::MAX as u64) as i32,
    }
}

fn empty_model<T: Clone + 'static>() -> ModelRc<T> {
    ModelRc::new(VecModel::default())
}

fn lan_error(message: String) -> crate::LanScoutOutputResult {
    crate::LanScoutOutputResult {
        ports: empty_model(),
        open_ports: empty_model(),
        hosts: empty_model(),
        error_message: message.into(),
        ..Default::default()
    }
}

fn lan_ports_output(results: Vec<crate::tools::lan_scout::PortScanResult>) -> crate::LanScoutOutputResult {
    let all: Vec<crate::PortItem> = results.iter().map(port_item).collect();
    let open: Vec<crate::PortItem> = all.iter().filter(|p| p.is_open).cloned().collect();
    crate::LanScoutOutputResult {
        open_count: open.len() as i32,
        total_count: all.len() as i32,
        ports: ModelRc::new(VecModel::from(all)),
        open_ports: ModelRc::new(VecModel::from(open)),
        hosts: empty_model(),
        hosts_probed: 0,
        error_message: "".into(),
    }
}

fn lan_hosts_output(hosts: Vec<crate::tools::lan_scout::HostResult>, probed: usize) -> crate::LanScoutOutputResult {
    let items: Vec<crate::HostItem> = hosts
        .into_iter()
        .map(|h| crate::HostItem {
            ip: h.ip.into(),
            latency_ms: h.latency_ms.min(i32::MAX as u64) as i32,
            open_ports: h.open_ports.into(),
            is_self: h.is_self,
            is_gateway: h.is_gateway,
        })
        .collect();
    crate::LanScoutOutputResult {
        open_count: items.len() as i32,
        total_count: probed as i32,
        ports: empty_model(),
        open_ports: empty_model(),
        hosts_probed: probed as i32,
        hosts: ModelRc::new(VecModel::from(items)),
        error_message: "".into(),
    }
}

fn tls_output(res: Result<crate::tools::tls_inspector::TlsReport, String>) -> crate::TlsOutputResult {
    match res {
        Ok(r) => {
            let chain: Vec<crate::TlsCertItem> = r
                .chain
                .into_iter()
                .map(|c| crate::TlsCertItem {
                    san_count: c.sans.len() as i32,
                    sans: c.sans.join(", ").into(),
                    common_name: c.common_name.into(),
                    subject: c.subject.into(),
                    issuer: c.issuer.into(),
                    serial: c.serial.into(),
                    not_before: c.not_before.into(),
                    not_after: c.not_after.into(),
                    days_left: c.days_left.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
                    signature_algorithm: c.signature_algorithm.into(),
                    public_key: c.public_key.into(),
                    sha256: c.sha256.into(),
                    is_ca: c.is_ca,
                })
                .collect();
            crate::TlsOutputResult {
                host: r.host.into(),
                port: r.port as i32,
                ip: r.ip.into(),
                protocol: r.protocol.into(),
                cipher_suite: r.cipher_suite.into(),
                alpn: r.alpn.into(),
                connect_ms: r.connect_ms.min(i32::MAX as u64) as i32,
                handshake_ms: r.handshake_ms.min(i32::MAX as u64) as i32,
                trusted: r.trusted,
                verify_error: r.verify_error.into(),
                hostname_matches: r.hostname_matches,
                chain: ModelRc::new(VecModel::from(chain)),
                error_message: "".into(),
            }
        }
        Err(e) => crate::TlsOutputResult { chain: empty_model(), error_message: e.into(), ..Default::default() },
    }
}

/// Push the detected network identity to the UI (callable from any thread).
fn publish_network(weak: &slint::Weak<crate::AppWindow>, net: &crate::tools::lan_scout::NetworkIdentity) {
    let device = net.device_ip.clone().unwrap_or_else(|| "unknown".into());
    let gateway = net.gateway_ip.clone().unwrap_or_else(|| "unknown".into());
    let mut parts = Vec::new();
    if let Some(i) = &net.interface {
        parts.push(i.clone());
    }
    if let Some(p) = net.prefix_len {
        parts.push(format!("/{p}"));
    }
    if !net.dns_servers.is_empty() {
        parts.push(format!("DNS {}", net.dns_servers.iter().take(3).cloned().collect::<Vec<_>>().join(", ")));
    }
    let summary = parts.join("  ·  ");
    let _ = weak.upgrade_in_event_loop(move |ui| {
        ui.set_detected_device_ip(device.into());
        ui.set_detected_gateway(gateway.into());
        ui.set_network_summary(summary.into());
    });
}

/// Detect the network on a worker thread (JNI + socket probes must not block startup).
fn refresh_network_async(weak: slint::Weak<crate::AppWindow>) {
    std::thread::spawn(move || {
        let net = crate::tools::lan_scout::detect_network_identity();
        publish_network(&weak, &net);
    });
}

fn set_suggestions(ui: &crate::AppWindow, state: &AppState, list: Vec<Suggestion>, source: &str) {
    let rows: Vec<crate::SmartSuggestion> = list
        .iter()
        .map(|s| crate::SmartSuggestion {
            tool_id: s.tool_id,
            tool_title: catalog::find(s.tool_id).map(|t| t.title).unwrap_or("Tool").into(),
            kind: s.kind.into(),
        })
        .collect();
    *state.suggestions.borrow_mut() = list;
    ui.set_smart_suggestions(ModelRc::new(VecModel::from(rows)));
    ui.set_smart_source(source.into());
}

fn show_feedback(ui: &crate::AppWindow, state: &AppState, text: &str) {
    ui.set_paste_feedback(text.into());
    let weak = ui.as_weak();
    state.feedback_timer.start(slint::TimerMode::SingleShot, std::time::Duration::from_millis(2800), move || {
        if let Some(ui) = weak.upgrade() {
            ui.set_paste_feedback("".into());
        }
    });
}

/// Open a tool. With `prefill`, the tool screen picks up the payload on creation.
fn open_tool(ui: &crate::AppWindow, state: &AppState, tool_id: i32, prefill: Option<(&str, i32)>) {
    if catalog::find(tool_id).is_none() {
        return;
    }
    crate::platform::android::haptic_tap();
    crate::storage::record_tool_opened(tool_id);
    state.nav.borrow_mut().open_tool(tool_id);
    match prefill {
        Some((text, mode)) => {
            ui.set_prefill_text(text.into());
            ui.set_prefill_mode(mode);
            ui.set_prefill_seq(ui.get_prefill_seq() + 1);
            ui.set_prefill_tool(tool_id);
        }
        // A stale prefill must not be re-applied the next time this tool is opened.
        None => ui.set_prefill_tool(0),
    }
    ui.set_about_open(false);
    ui.set_modal_open(false);
    ui.set_active_tool(tool_id);
    state.refresh_lists();
}

fn apply_back(ui: &crate::AppWindow, state: &AppState) -> bool {
    if ui.get_welcome_open() {
        let page = ui.get_welcome_page();
        if page > 0 {
            ui.set_welcome_page(page - 1);
            return true;
        }
        return false;
    }
    let outcome = state.nav.borrow_mut().handle_back();
    match outcome {
        BackNavigationOutcome::DismissModal => {
            ui.set_modal_open(false);
            ui.set_about_open(false);
            true
        }
        BackNavigationOutcome::CloseTool { return_to } => {
            ui.set_active_tool(0);
            ui.set_current_mode(return_to.id());
            true
        }
        BackNavigationOutcome::NavigateToHome => {
            ui.set_active_tool(0);
            ui.set_current_mode(AppMode::Home.id());
            true
        }
        BackNavigationOutcome::ExitApp => false,
    }
}

pub fn setup_app_state(ui: &crate::AppWindow) {
    let state = AppState::new();
    update_theme_settings(ui);
    update_tool_preferences(ui);
    ui.set_home_tools(ModelRc::from(state.home.clone()));
    ui.set_favorite_tools(ModelRc::from(state.favorites.clone()));
    ui.set_recent_tools(ModelRc::from(state.recents.clone()));
    state.refresh_lists();
    setup_tool_handlers(ui);
    refresh_network_async(ui.as_weak());

    let (st, weak) = (state.clone(), ui.as_weak());
    ui.on_switch_mode(move |mode_id| {
        let Some(ui) = weak.upgrade() else { return };
        let mode = AppMode::from_id(mode_id);
        if mode.id() != ui.get_current_mode() || ui.get_active_tool() != 0 {
            crate::platform::android::haptic_tap();
        }
        st.nav.borrow_mut().switch_mode(mode);
        ui.set_current_mode(mode.id());
        ui.set_active_tool(0);
        ui.set_about_open(false);
        ui.set_modal_open(false);
    });

    let (st, weak) = (state.clone(), ui.as_weak());
    ui.on_open_tool(move |tool_id| {
        if let Some(ui) = weak.upgrade() {
            open_tool(&ui, &st, tool_id, None);
        }
    });

    let (st, weak) = (state.clone(), ui.as_weak());
    ui.on_open_tool_with(move |tool_id, payload| {
        if let Some(ui) = weak.upgrade() {
            open_tool(&ui, &st, tool_id, Some((payload.as_str(), 0)));
        }
    });

    let st = state.clone();
    ui.on_toggle_favorite(move |tool_id| {
        crate::platform::android::haptic_tap();
        crate::storage::toggle_favorite(tool_id);
        st.refresh_lists();
    });

    let st = state.clone();
    ui.on_clear_recents(move || {
        crate::storage::clear_recents();
        st.refresh_lists();
    });

    let st = state.clone();
    ui.on_clear_favorites(move || {
        crate::storage::clear_favorites();
        st.refresh_lists();
    });

    // Search & Smart Paste
    let (st, weak) = (state.clone(), ui.as_weak());
    ui.on_search_edited(move |text| {
        let Some(ui) = weak.upgrade() else { return };
        *st.query.borrow_mut() = text.to_string();
        st.refresh_lists();
        // Typed or pasted data (a JWT, a URL, an IP...) is routed like Smart Paste, but only
        // confident matches are shown so ordinary searches are not cluttered.
        let detected: Vec<Suggestion> = if text.trim().chars().count() >= 4 {
            smart_detect::detect(&text).into_iter().filter(|s| s.score >= 70).collect()
        } else {
            Vec::new()
        };
        set_suggestions(&ui, &st, detected, "search text");
    });

    let (st, weak) = (state.clone(), ui.as_weak());
    ui.on_clear_search(move || {
        let Some(ui) = weak.upgrade() else { return };
        st.query.borrow_mut().clear();
        ui.set_search_query("".into());
        set_suggestions(&ui, &st, Vec::new(), "");
        st.refresh_lists();
    });

    let (st, weak) = (state.clone(), ui.as_weak());
    ui.on_smart_paste(move || {
        let Some(ui) = weak.upgrade() else { return };
        crate::platform::android::haptic_tap();
        let clip = crate::platform::android::read_clipboard().unwrap_or_default();
        let clip = clip.trim();
        if clip.is_empty() {
            set_suggestions(&ui, &st, Vec::new(), "");
            show_feedback(&ui, &st, "Clipboard is empty — copy something first");
            return;
        }
        let found = smart_detect::detect(clip);
        let chars = clip.chars().count();
        let source = format!("clipboard · {chars} char{}", if chars == 1 { "" } else { "s" });
        set_suggestions(&ui, &st, found, &source);
        ui.set_paste_feedback("".into());
    });

    let (st, weak) = (state.clone(), ui.as_weak());
    ui.on_open_suggestion(move |i| {
        let Some(ui) = weak.upgrade() else { return };
        let picked = st.suggestions.borrow().get(i.max(0) as usize).cloned();
        let Some(s) = picked else { return };
        // Routing done: reset the omnibox so Home shows the full grid on return.
        st.query.borrow_mut().clear();
        ui.set_search_query("".into());
        set_suggestions(&ui, &st, Vec::new(), "");
        open_tool(&ui, &st, s.tool_id, Some((s.payload.as_str(), prefill_mode_for(&s))));
    });

    // Settings
    let weak = ui.as_weak();
    ui.on_dark_theme_changed(move |enabled| {
        crate::platform::android::haptic_tap();
        crate::storage::save_dark_theme(enabled);
        if let Some(ui) = weak.upgrade() {
            ui.set_dark_theme_enabled(enabled);
        }
    });

    let weak = ui.as_weak();
    ui.on_dynamic_color_changed(move |enabled| {
        crate::platform::android::haptic_tap();
        crate::storage::save_dynamic_color(enabled);
        if let Some(ui) = weak.upgrade() {
            ui.set_dynamic_color_enabled(enabled);
        }
    });

    let weak = ui.as_weak();
    ui.on_default_hash_algorithm_changed(move |algo| {
        crate::storage::save_default_hash_algorithm(algo);
        if let Some(ui) = weak.upgrade() {
            ui.set_default_hash_algorithm(algo);
        }
    });

    let weak = ui.as_weak();
    ui.on_default_uuid_version_changed(move |ver| {
        crate::storage::save_default_uuid_version(ver);
        if let Some(ui) = weak.upgrade() {
            ui.set_default_uuid_version(ver);
        }
    });

    let weak = ui.as_weak();
    ui.on_default_timestamp_format_changed(move |fmt| {
        crate::storage::save_default_timestamp_format(fmt);
        if let Some(ui) = weak.upgrade() {
            ui.set_default_timestamp_format(fmt);
        }
    });

    let weak = ui.as_weak();
    ui.on_graphics_backend_changed(move |value| {
        crate::platform::android::haptic_tap();
        crate::storage::save_graphics_backend(value);
        if let Some(ui) = weak.upgrade() {
            ui.set_graphics_backend(value);
        }
    });

    let weak = ui.as_weak();
    ui.on_peak_refresh_changed(move |enabled| {
        crate::platform::android::haptic_tap();
        crate::storage::save_high_refresh_rate(enabled);
        crate::platform::android::apply_refresh_rate_setting(enabled);
        if let Some(ui) = weak.upgrade() {
            ui.set_peak_refresh_enabled(enabled);
        }
    });

    let (st, weak) = (state.clone(), ui.as_weak());
    ui.on_open_about(move || {
        st.nav.borrow_mut().open_modal();
        if let Some(ui) = weak.upgrade() {
            ui.set_about_open(true);
            ui.set_modal_open(true);
        }
    });

    let weak = ui.as_weak();
    ui.on_dismiss_welcome(move || {
        crate::storage::save_has_seen_welcome(true);
        if let Some(ui) = weak.upgrade() {
            ui.set_welcome_open(false);
            ui.set_welcome_page(0);
        }
    });

    let weak = ui.as_weak();
    ui.on_reset_welcome(move || {
        crate::platform::android::haptic_tap();
        crate::storage::reset_welcome();
        if let Some(ui) = weak.upgrade() {
            ui.set_welcome_page(0);
        }
    });

    let (st, weak) = (state.clone(), ui.as_weak());
    ui.on_back(move || {
        if let Some(ui) = weak.upgrade() {
            apply_back(&ui, &st);
        }
    });

    let (st, weak) = (state.clone(), ui.as_weak());
    ui.on_handle_back(move || weak.upgrade().map(|ui| apply_back(&ui, &st)).unwrap_or(false));
}

#[cfg(target_os = "android")]
pub fn android_main(android_app: slint::android::AndroidApp) {
    if let Some(path) = android_app.internal_data_path() {
        crate::storage::init_storage_dir(path.to_path_buf());
    }
    crate::platform::android::init(&android_app);
    crate::platform::android::configure_window_soft_input_mode();
    let status_inset = crate::platform::android::query_status_bar_inset();
    let nav_inset = crate::platform::android::query_navigation_bar_inset();
    crate::platform::android::set_status_bar_inset(status_inset);
    crate::platform::android::set_navigation_bar_inset(nav_inset);

    slint::android::init(android_app).expect("Slint Android backend");

    // The renderer must be chosen before the window is created. Slint's Android backend
    // ignores SLINT_BACKEND, which is why the previous "OpenGL on Tensor G5" switch never
    // took effect. Vulkan is the default candidate order (Vulkan, then GL fallback), so it
    // only needs an explicit request for OpenGL ES.
    let api = crate::platform::android::resolve_graphics_api(crate::storage::load_graphics_backend());
    if api == crate::platform::android::GraphicsApi::OpenGlEs {
        if let Err(e) = slint::BackendSelector::new().require_opengl_es().select() {
            eprintln!("DevKit: could not request OpenGL ES ({e}); using default renderer");
        }
    }

    let ui = crate::AppWindow::new().expect("create main window");
    ui.set_status_bar_fallback(status_inset);
    ui.set_nav_bar_fallback(nav_inset);
    ui.set_welcome_open(!crate::storage::load_has_seen_welcome());
    crate::MaterialWindowAdapter::get(&ui).set_disable_hover(true);
    setup_app_state(&ui);
    crate::platform::android::apply_refresh_rate_setting(crate::storage::load_high_refresh_rate());
    ui.run().expect("run event loop");
}
