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
}

impl Screen {
    pub const ALL: [Screen; 7] = [
        Screen::Home,
        Screen::Subnet,
        Screen::Hash,
        Screen::Base64,
        Screen::Uuid,
        Screen::Timestamp,
        Screen::Regex,
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
            _ => Screen::Home,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct NavigationState {
    pub current_screen: Screen,
}

impl NavigationState {
    pub fn new() -> Self {
        Self {
            current_screen: Screen::Home,
        }
    }

    pub fn navigate_to(&mut self, screen: Screen) {
        self.current_screen = screen;
    }

    pub fn navigate_back(&mut self) -> Screen {
        self.current_screen = Screen::Home;
        self.current_screen
    }
}
