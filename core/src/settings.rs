//! Ustawienia programu (settings.json).

use serde::{Deserialize, Serialize};

use crate::audio::dsp::DspSettings;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    // Wygląd
    pub theme: String,
    pub accent: String,
    pub accent_from_cover: bool,
    pub viz: i32,
    pub viz_bands: i32,
    pub viz_sensitivity: f32,
    pub show_waveform: bool,
    /// Styl taśmy na pasku przewijania: 0 – fala, 1 – słupki, 2 – fala jednostronna, 3 – kontur.
    pub wave_style: i32,
    pub group_albums: bool,
    pub show_remaining: bool,
    pub compact: bool,
    pub always_on_top: bool,
    pub show_lyrics: bool,
    /// Pobieranie brakujących tekstów z LRCLIB.
    pub online_lyrics: bool,
    pub show_sidebar: bool,
    pub scroll_title: bool,
    // Dźwięk
    pub volume: f32,
    pub muted: bool,
    pub device: Option<String>,
    pub crossfade: f64,
    pub soft_pause: bool,
    /// 0 – wyłączone, 1 – utwór, 2 – album.
    pub replaygain: i32,
    pub rg_preamp: f32,
    pub speed: f64,
    pub dsp: DspSettings,
    pub eq_preset: String,
    /// Korektor dobierany do gatunku bieżącego utworu.
    pub auto_eq: bool,
    // Odtwarzanie
    pub shuffle: bool,
    /// 0 – bez powtarzania, 1 – lista, 2 – utwór.
    pub repeat: i32,
    pub resume_on_start: bool,
    pub follow_playing: bool,
    pub seek_step: f64,
    // Wideo
    pub hwdec: bool,
    pub video_resume: bool,
    pub autohide_ms: i32,
    pub video_fit_window: bool,
    pub sub_size: i32,
    pub sub_lang: String,
    pub audio_lang: String,
    pub screenshot_dir: String,
    pub deinterlace: bool,
    // System
    pub single_instance: bool,
    pub media_keys: bool,
    pub prevent_sleep: bool,
    pub check_updates: bool,
    pub library_folders: Vec<String>,
    pub video_folders: Vec<String>,
    pub custom_eq: Vec<(String, Vec<f32>)>,
    pub skip_version: String,
}

impl Default for Settings {
    fn default() -> Self {
        let pictures = dirs::picture_dir().map(|p| p.join("OpenMP").to_string_lossy().into_owned()).unwrap_or_default();
        Settings {
            theme: "dark".into(),
            accent: "#4f8cff".into(),
            accent_from_cover: true,
            viz: 0,
            viz_bands: 48,
            viz_sensitivity: 0.0,
            show_waveform: true,
            wave_style: 1,
            group_albums: true,
            show_remaining: false,
            compact: false,
            always_on_top: false,
            show_lyrics: false,
            online_lyrics: true,
            show_sidebar: true,
            scroll_title: true,
            volume: 0.8,
            muted: false,
            device: None,
            crossfade: 0.0,
            soft_pause: true,
            replaygain: 1,
            rg_preamp: 0.0,
            speed: 1.0,
            dsp: DspSettings::default(),
            eq_preset: "Płasko".into(),
            auto_eq: false,
            shuffle: false,
            repeat: 0,
            resume_on_start: true,
            follow_playing: true,
            seek_step: 5.0,
            hwdec: true,
            video_resume: true,
            autohide_ms: 2200,
            video_fit_window: true,
            sub_size: 46,
            sub_lang: "pol,pl,eng,en".into(),
            audio_lang: "pol,pl,eng,en".into(),
            screenshot_dir: pictures,
            deinterlace: false,
            single_instance: true,
            media_keys: true,
            prevent_sleep: true,
            check_updates: true,
            library_folders: dirs::audio_dir().map(|p| vec![p.to_string_lossy().into_owned()]).unwrap_or_default(),
            video_folders: dirs::video_dir().map(|p| vec![p.to_string_lossy().into_owned()]).unwrap_or_default(),
            custom_eq: Vec::new(),
            skip_version: String::new(),
        }
    }
}

impl Settings {
    pub fn load() -> Settings {
        crate::paths::load_json("settings.json")
    }
    pub fn save(&self) {
        crate::paths::save_json("settings.json", self);
    }
}

/// Stan sesji: co grało i gdzie stało okno (state.json).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub list_id: u64,
    pub idx: usize,
    pub path: String,
    pub pos: f64,
    pub window: Option<(i32, i32, u32, u32)>,
    pub maximized: bool,
}

impl Session {
    pub fn load() -> Session {
        crate::paths::load_json("state.json")
    }
    pub fn save(&self) {
        crate::paths::save_json("state.json", self);
    }
}
