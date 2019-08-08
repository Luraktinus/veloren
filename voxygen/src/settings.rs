use crate::{hud::CrosshairType, ui::ScaleMode, window::DigitalInput};
use directories::ProjectDirs;
use glutin::{MouseButton, VirtualKeyCode};
use log::warn;
use serde_derive::{Deserialize, Serialize};
use std::{fs, io::prelude::*, path::PathBuf};

/// `ControlSettings` contains keybindings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlSettings {
    pub toggle_cursor: DigitalInput,
    pub escape: DigitalInput,
    pub enter: DigitalInput,
    pub command: DigitalInput,
    pub move_forward: DigitalInput,
    pub move_left: DigitalInput,
    pub move_back: DigitalInput,
    pub move_right: DigitalInput,
    pub jump: DigitalInput,
    pub glide: DigitalInput,
    pub map: DigitalInput,
    pub bag: DigitalInput,
    pub quest_log: DigitalInput,
    pub character_window: DigitalInput,
    pub social: DigitalInput,
    pub spellbook: DigitalInput,
    pub settings: DigitalInput,
    pub help: DigitalInput,
    pub toggle_interface: DigitalInput,
    pub toggle_debug: DigitalInput,
    pub fullscreen: DigitalInput,
    pub screenshot: DigitalInput,
    pub toggle_ingame_ui: DigitalInput,
    pub attack: DigitalInput,
    pub second_attack: DigitalInput,
    pub roll: DigitalInput,
    pub interact: DigitalInput,
}

impl Default for ControlSettings {
    fn default() -> Self {
        Self {
            toggle_cursor: DigitalInput::Key(VirtualKeyCode::Tab),
            escape: DigitalInput::Key(VirtualKeyCode::Escape),
            enter: DigitalInput::Key(VirtualKeyCode::Return),
            command: DigitalInput::Key(VirtualKeyCode::Slash),
            move_forward: DigitalInput::Key(VirtualKeyCode::W),
            move_left: DigitalInput::Key(VirtualKeyCode::A),
            move_back: DigitalInput::Key(VirtualKeyCode::S),
            move_right: DigitalInput::Key(VirtualKeyCode::D),
            jump: DigitalInput::Key(VirtualKeyCode::Space),
            glide: DigitalInput::Key(VirtualKeyCode::LShift),
            map: DigitalInput::Key(VirtualKeyCode::M),
            bag: DigitalInput::Key(VirtualKeyCode::B),
            quest_log: DigitalInput::Key(VirtualKeyCode::L),
            character_window: DigitalInput::Key(VirtualKeyCode::C),
            social: DigitalInput::Key(VirtualKeyCode::O),
            spellbook: DigitalInput::Key(VirtualKeyCode::P),
            settings: DigitalInput::Key(VirtualKeyCode::N),
            help: DigitalInput::Key(VirtualKeyCode::F1),
            toggle_interface: DigitalInput::Key(VirtualKeyCode::F2),
            toggle_debug: DigitalInput::Key(VirtualKeyCode::F3),
            fullscreen: DigitalInput::Key(VirtualKeyCode::F11),
            screenshot: DigitalInput::Key(VirtualKeyCode::F4),
            toggle_ingame_ui: DigitalInput::Key(VirtualKeyCode::F6),
            attack: DigitalInput::Mouse(MouseButton::Left),
            second_attack: DigitalInput::Mouse(MouseButton::Right),
            roll: DigitalInput::Mouse(MouseButton::Middle),
            interact: DigitalInput::Key(VirtualKeyCode::E),
        }
    }
}

/// `GameplaySettings` contains sensitivity and gameplay options.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GameplaySettings {
    pub pan_sensitivity: u32,
    pub zoom_sensitivity: u32,
    pub crosshair_transp: f32,
    pub crosshair_type: CrosshairType,
    pub ui_scale: ScaleMode,
}

impl Default for GameplaySettings {
    fn default() -> Self {
        Self {
            pan_sensitivity: 100,
            zoom_sensitivity: 100,
            crosshair_transp: 0.6,
            crosshair_type: CrosshairType::Round,
            ui_scale: ScaleMode::RelativeToWindow([1920.0, 1080.0].into()),
        }
    }
}

/// `NetworkingSettings` stores server and networking settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkingSettings {
    pub username: String,
    pub servers: Vec<String>,
    pub default_server: usize,
}

impl Default for NetworkingSettings {
    fn default() -> Self {
        Self {
            username: "Username".to_string(),
            servers: vec!["server.veloren.net".to_string()],
            default_server: 0,
        }
    }
}

/// `Log` stores the name to the log file.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Log {
    pub file: PathBuf,
}

impl Default for Log {
    fn default() -> Self {
        Self {
            file: "voxygen.log".into(),
        }
    }
}

/// `GraphicsSettings` contains settings related to framerate and in-game visuals.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphicsSettings {
    pub view_distance: u32,
    pub max_fps: u32,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            view_distance: 5,
            max_fps: 60,
        }
    }
}

/// `AudioSettings` controls the volume of different audio subsystems and which
/// device is used.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,

    /// Audio Device that Voxygen will use to play audio.
    pub audio_device: Option<String>,
    pub audio_on: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            music_volume: 0.5,
            sfx_volume: 0.5,
            audio_device: None,
            audio_on: true,
        }
    }
}

/// `Settings` contains everything that can be configured in the settings.ron file.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub controls: ControlSettings,
    pub gameplay: GameplaySettings,
    pub networking: NetworkingSettings,
    pub log: Log,
    pub graphics: GraphicsSettings,
    pub audio: AudioSettings,
    pub show_disclaimer: bool,
    pub send_logon_commands: bool,
    // TODO: Remove at a later date, for dev testing
    pub logon_commands: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            controls: ControlSettings::default(),
            gameplay: GameplaySettings::default(),
            networking: NetworkingSettings::default(),
            log: Log::default(),
            graphics: GraphicsSettings::default(),
            audio: AudioSettings::default(),
            show_disclaimer: true,
            send_logon_commands: false,
            logon_commands: Vec::new(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = Settings::get_settings_path();

        // If file doesn't exist, use the default settings.
        if let Ok(file) = fs::File::open(path) {
            ron::de::from_reader(file).expect("Error parsing settings")
        } else {
            Self::default()
        }
    }

    pub fn save_to_file_warn(&self) {
        if let Err(err) = self.save_to_file() {
            warn!("Failed to save settings: {:?}", err);
        }
    }

    pub fn save_to_file(&self) -> std::io::Result<()> {
        let path = Settings::get_settings_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut config_file = fs::File::create(path)?;

        let s: &str = &ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap();
        config_file.write_all(s.as_bytes()).unwrap();
        Ok(())
    }

    fn get_settings_path() -> PathBuf {
        let proj_dirs = ProjectDirs::from("net", "veloren", "voxygen")
            .expect("System's $HOME directory path not found!");
        proj_dirs
            .config_dir()
            .join("settings")
            .with_extension("ron")
    }
}
