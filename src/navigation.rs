// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 MTRE Consulting

//! Navigation state and routing models.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppMode {
    #[default]
    Home = 0,
    Recent = 1,
    Favorites = 2,
    Settings = 3,
}

impl AppMode {
    pub const ALL: [AppMode; 4] = [
        AppMode::Home,
        AppMode::Recent,
        AppMode::Favorites,
        AppMode::Settings,
    ];

    pub fn id(&self) -> i32 {
        *self as i32
    }

    pub fn from_id(id: i32) -> Self {
        match id {
            1 => AppMode::Recent,
            2 => AppMode::Favorites,
            3 => AppMode::Settings,
            _ => AppMode::Home,
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            AppMode::Home => "DevKit",
            AppMode::Recent => "Recent Tools",
            AppMode::Favorites => "Favorites",
            AppMode::Settings => "Settings",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Home,
    Subnet,
    Hash,
    Base64,
    Uuid,
    Timestamp,
    Regex,
    JsonYaml,
    Cron,
    Gzip,
    Formatter,
    Chmod,
    Color,
    Contrast,
    Jwt,
}

impl Screen {
    pub const ALL: [Screen; 15] = [
        Screen::Home,
        Screen::Subnet,
        Screen::Hash,
        Screen::Base64,
        Screen::Uuid,
        Screen::Timestamp,
        Screen::Regex,
        Screen::JsonYaml,
        Screen::Cron,
        Screen::Gzip,
        Screen::Formatter,
        Screen::Chmod,
        Screen::Color,
        Screen::Contrast,
        Screen::Jwt,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            Screen::Home => "DevKit",
            Screen::Subnet => "Subnet Calculator",
            Screen::Hash => "Hash Calculator",
            Screen::Base64 => "Base64 Encoder",
            Screen::Uuid => "UUID Generator",
            Screen::Timestamp => "Timestamp Converter",
            Screen::Regex => "Regex Tester",
            Screen::JsonYaml => "JSON ↔ YAML Converter",
            Screen::Cron => "CRON Parser",
            Screen::Gzip => "GZip Compressor",
            Screen::Formatter => "Code Formatter",
            Screen::Chmod => "Chmod Calculator",
            Screen::Color => "Color Converter",
            Screen::Contrast => "Contrast Checker",
            Screen::Jwt => "JWT Decoder",
        }
    }

    pub fn id(&self) -> i32 {
        match self {
            Screen::Home => 0,
            Screen::Subnet => 1,
            Screen::Hash => 2,
            Screen::Base64 => 3,
            Screen::Uuid => 4,
            Screen::Timestamp => 5,
            Screen::Regex => 6,
            Screen::JsonYaml => 7,
            Screen::Cron => 8,
            Screen::Gzip => 9,
            Screen::Formatter => 10,
            Screen::Chmod => 11,
            Screen::Color => 12,
            Screen::Contrast => 13,
            Screen::Jwt => 14,
        }
    }

    pub fn from_id(id: i32) -> Self {
        match id {
            1 => Screen::Subnet,
            2 => Screen::Hash,
            3 => Screen::Base64,
            4 => Screen::Uuid,
            5 => Screen::Timestamp,
            6 => Screen::Regex,
            7 => Screen::JsonYaml,
            8 => Screen::Cron,
            9 => Screen::Gzip,
            10 => Screen::Formatter,
            11 => Screen::Chmod,
            12 => Screen::Color,
            13 => Screen::Contrast,
            14 => Screen::Jwt,
            _ => Screen::Home,
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackNavigationOutcome {
    DismissModal,
    CloseTool { return_to: AppMode },
    NavigateToHome,
    ExitApp,
}

#[derive(Debug, Clone, Default)]
pub struct NavigationState {
    pub current_mode: AppMode,
    pub active_tool: i32,
    pub tool_origin_mode: AppMode,
    pub modal_open: bool,
}

impl NavigationState {
    pub fn new() -> Self {
        Self {
            current_mode: AppMode::Home,
            active_tool: 0,
            tool_origin_mode: AppMode::Home,
            modal_open: false,
        }
    }

    pub fn switch_mode(&mut self, mode: AppMode) {
        self.current_mode = mode;
        self.active_tool = 0;
    }

    pub fn open_tool(&mut self, tool_id: i32) {
        self.tool_origin_mode = self.current_mode;
        self.active_tool = tool_id;
    }

    pub fn close_tool(&mut self) -> AppMode {
        self.active_tool = 0;
        self.current_mode = self.tool_origin_mode;
        self.tool_origin_mode
    }

    pub fn open_modal(&mut self) {
        self.modal_open = true;
    }

    pub fn dismiss_modal(&mut self) {
        self.modal_open = false;
    }

    pub fn handle_back(&mut self) -> BackNavigationOutcome {
        if self.modal_open {
            self.modal_open = false;
            return BackNavigationOutcome::DismissModal;
        }
        if self.active_tool > 0 {
            let return_to = self.close_tool();
            return BackNavigationOutcome::CloseTool { return_to };
        }
        if self.current_mode != AppMode::Home {
            self.current_mode = AppMode::Home;
            return BackNavigationOutcome::NavigateToHome;
        }
        BackNavigationOutcome::ExitApp
    }
}
