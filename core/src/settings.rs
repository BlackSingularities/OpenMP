//! Ustawienia programu (settings.json).

use serde::{Deserialize, Serialize};

use crate::audio::dsp::DspSettings;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    // Wygląd
    pub theme: String,
    /// Język interfejsu (kod, np. "pl"); puste – język systemu.
    pub language: String,
    /// Tytuł i podtytuł na dole okładek w siatce zamiast pod nimi.
    pub caption_overlay: bool,
    pub accent: String,
    pub accent_from_cover: bool,
    pub viz: i32,
    pub viz_bands: i32,
    pub viz_sensitivity: f32,
    pub show_waveform: bool,
    /// Styl taśmy na pasku przewijania: 0 – fala, 1 – słupki, 2 – fala jednostronna, 3 – kontur.
    pub wave_style: i32,
    /// Żywa część taśmy przy znaczniku (0 – wyłączona), jej zasięg (0..2) i poświata.
    pub live_style: i32,
    pub live_span: i32,
    pub live_glow: bool,
    pub group_albums: bool,
    pub show_remaining: bool,
    pub compact: bool,
    pub always_on_top: bool,
    pub show_lyrics: bool,
    /// Pobieranie brakujących tekstów z LRCLIB.
    pub online_lyrics: bool,
    /// Pobieranie brakujących okładek z MusicBrainz / Cover Art Archive.
    pub online_covers: bool,
    /// Token ListenBrainz – pusty wyłącza scrobbling.
    pub listenbrainz_token: String,
    /// Klucze API: TMDB (plakaty i opisy po polsku) i OpenSubtitles (napisy).
    pub tmdb_key: String,
    pub opensubs_key: String,
    pub show_sidebar: bool,
    pub scroll_title: bool,
    /// Zasięg koloru przewodniego: 0 – tylko akcenty, 1 – też ikony, suwaki i znaczniki, 2 – cały interfejs.
    pub accent_scope: i32,
    /// Tło okna z okładki: 0 – brak, 1 – rozmyta okładka, 2 – gradient barw okładki.
    pub backdrop: i32,
    pub backdrop_opacity: f32,
    /// Powolny ruch tła (kosztuje ciągłe odświeżanie okna).
    pub backdrop_motion: bool,
    /// Siła poświaty akcentu (0–2).
    pub glow: f32,
    /// Zaokrąglenia: 0 – ostre, 1 – zwykłe, 2 – mocne.
    pub corners: i32,
    pub text_scale: f32,
    /// Przezroczystość paneli (0 – pełne, do 0.8) – prześwituje rozmyte tło z okładki.
    pub panel_glass: f32,
    /// Barwa szkła wszystkich okien i paneli: 0 – neutralna, 1 – w kolorze akcentu, 2 – czerń.
    pub glass_tint: i32,
    /// Wielkość tekstu panelu nawigacji względem reszty.
    pub nav_text: f32,
    /// Czcionka tytułu: 0 – szeryfowa, 1 – bezszeryfowa, 2 – techniczna.
    pub title_font: i32,
    /// Kompaktowy interfejs: niższe wiersze i mniejsze odstępy.
    pub compact_ui: bool,
    /// Utwory jako siatka okładek zamiast listy i szerokość kafelka.
    pub track_grid: bool,
    pub grid_size: f32,
    // Dźwięk
    pub volume: f32,
    pub muted: bool,
    pub device: Option<String>,
    pub crossfade: f64,
    pub soft_pause: bool,
    /// Tryb wyłączny WASAPI (karta w częstotliwości pliku, bez miksera Windows).
    pub exclusive: bool,
    /// Jakość przepróbkowania: 0 – szybka, 1 – dobra, 2 – najlepsza.
    pub resample_quality: i32,
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
    pub sub_size: i32,
    pub sub_lang: String,
    pub audio_lang: String,
    pub screenshot_dir: String,
    pub deinterlace: bool,
    /// Wyjście z widoku filmu: 0 – obraz dalej gra w oknie programu, 1 – tylko dźwięk, 2 – pauza.
    pub video_leave: i32,
    /// Nowy film od razu na pełnym ekranie (inaczej gra w oknie, w miejscu okładki obok opisu).
    /// Nowy klucz – dawne „otwieraj widok filmu” (domyślnie włączone) nie przechodzi.
    #[serde(rename = "video_start_fullscreen")]
    pub video_open_view: bool,
    /// Chowanie sterowania także w oknie (na pełnym ekranie zawsze).
    pub autohide_window: bool,
    /// Jakość obrazu: 0 – szybka, 1 – zrównoważona, 2 – wysoka, 3 – najwyższa.
    pub video_quality: i32,
    /// Dekodowanie sprzętowe: 0 – wyłączone, 1 – auto, 2 – auto z kopiowaniem, 3 – D3D11, 4 – NVDEC, 5 – DXVA2.
    pub hwdec_mode: i32,
    pub interpolation: bool,
    pub deband: bool,
    /// Mapowanie tonów HDR: 0 – auto, 1 – BT.2390, 2 – Hable, 3 – Mobius, 4 – Reinhard, 5 – obcięcie.
    pub tone_mapping: i32,
    /// Poświata z obrazu wokół filmu: 0 – wyłączona, 1 – delikatna, 2 – mocna.
    pub ambient: i32,
    /// Kolor przewodni z barw filmu.
    pub video_tint: bool,
    /// Jak często kolor z obrazu: 0 – 0,5 s, 1 – 1,2 s, 2 – 3 s, 3 – 8 s.
    pub tint_rate: i32,
    /// Podgląd klatek nad paskiem przewijania.
    pub seek_preview: bool,
    /// Kółko myszy nad obrazem: 0 – głośność, 1 – przewijanie.
    pub wheel_action: i32,
    /// Dwuklik przy lewej/prawej krawędzi obrazu przewija o 10 s.
    pub dblclick_seek: bool,
    /// Po końcu filmu następny plik z listy.
    pub video_autonext: bool,
    /// Od jakiej części długości utwór liczy się jako odsłuchany, a film jako obejrzany (%).
    pub played_pct: f32,
    /// Wzmocnienie dźwięku filmów (100–200%).
    pub volume_boost: f32,
    /// Tryb nocny: wyrównanie dynamiki (ciche dialogi, głośne wybuchy).
    pub night_mode: bool,
    /// Miks dźwięku wielokanałowego do stereo.
    pub downmix: bool,
    /// Jakość filmów z sieci (yt-dlp): 0 – najlepsza, 1 – do 1080p, 2 – do 720p, 3 – do 480p.
    pub ytdl_format: i32,
    pub ytdl: bool,
    /// Bufor strumieni w MB.
    pub cache_mb: i32,
    // Napisy
    pub sub_font: String,
    /// 0 – biały, 1 – żółty, 2 – błękitny, 3 – zielony.
    pub sub_color: i32,
    /// 0 – obrys, 1 – cień, 2 – półprzezroczyste pole.
    pub sub_style: i32,
    /// Położenie napisów (100 – dół obrazu).
    pub sub_pos: i32,
    pub sub_bold: bool,
    /// Wymuszanie własnego stylu także na napisach ASS.
    pub sub_override: bool,
    /// Wczytywanie napisów obok filmu: 0 – nie, 1 – ta sama nazwa, 2 – podobna nazwa, 3 – wszystkie z folderu.
    pub sub_auto: i32,
    // System
    pub single_instance: bool,
    /// Ikona w zasobniku systemowym; zamknięcie okna chowa je tam.
    pub tray: bool,
    /// Własne skróty: (kombinacja, np. „Ctrl+Shift+K”, id akcji) – mają pierwszeństwo przed domyślnymi.
    pub custom_keys: Vec<(String, String)>,
    /// Subskrybowane podcasty: (adres kanału RSS, tytuł).
    pub podcasts: Vec<(String, String)>,
    /// Ekran powitalny już zamknięty (pokazywany tylko przy pierwszym uruchomieniu).
    pub welcome_done: bool,
    pub media_keys: bool,
    /// Globalne skróty Ctrl+Alt+Spacja/strzałki.
    pub global_hotkeys: bool,
    pub prevent_sleep: bool,
    pub check_updates: bool,
    pub library_folders: Vec<String>,
    pub video_folders: Vec<String>,
    pub custom_eq: Vec<(String, Vec<f32>)>,
    /// Inteligentne listy: nazwa i zapytanie z filtrami (`library::Query`).
    pub smart_lists: Vec<(String, String)>,
    /// Zwinięte sekcje panelu nawigacji (identyfikatory „sec-…”).
    pub nav_collapsed: Vec<String>,
    /// Sekcje panelu nawigacji wyłączone w ustawieniach („playlist” albo „sec-…”).
    pub nav_hidden: Vec<String>,
    /// Szerokość panelu nawigacji (px), zmieniana przeciąganiem krawędzi.
    pub sidebar_width: f32,
    pub skip_version: String,
    /// Wersja ustawień dźwięku – stare domyślne (ReplayGain ściszający utwory) są poprawiane raz.
    #[serde(default)]
    pub audio_rev: u32,
}

impl Default for Settings {
    fn default() -> Self {
        let pictures = dirs::picture_dir().map(|p| p.join("OpenMP").to_string_lossy().into_owned()).unwrap_or_default();
        Settings {
            theme: "dark".into(),
            language: String::new(),
            caption_overlay: false,
            accent: "#4f8cff".into(),
            accent_from_cover: true,
            viz: 0,
            viz_bands: 48,
            viz_sensitivity: 0.0,
            show_waveform: true,
            wave_style: 1,
            live_style: 1,
            live_span: 1,
            live_glow: true,
            group_albums: true,
            show_remaining: false,
            compact: false,
            always_on_top: false,
            show_lyrics: false,
            online_lyrics: true,
            online_covers: true,
            listenbrainz_token: String::new(),
            tmdb_key: String::new(),
            opensubs_key: String::new(),
            show_sidebar: true,
            scroll_title: true,
            accent_scope: 1,
            backdrop: 2,
            backdrop_opacity: 0.2,
            backdrop_motion: false,
            glow: 1.0,
            corners: 1,
            text_scale: 1.0,
            panel_glass: 0.0,
            glass_tint: 0,
            nav_text: 1.0,
            title_font: 0,
            compact_ui: false,
            track_grid: false,
            grid_size: 160.0,
            volume: 0.8,
            muted: false,
            device: None,
            crossfade: 0.0,
            soft_pause: true,
            exclusive: false,
            resample_quality: 1,
            replaygain: 0,
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
            sub_size: 46,
            sub_lang: "pol,pl,eng,en".into(),
            audio_lang: "pol,pl,eng,en".into(),
            screenshot_dir: pictures,
            deinterlace: false,
            video_leave: 0,
            video_open_view: false,
            autohide_window: true,
            video_quality: 1,
            hwdec_mode: 1,
            interpolation: false,
            deband: false,
            tone_mapping: 0,
            ambient: 1,
            video_tint: true,
            tint_rate: 1,
            seek_preview: true,
            wheel_action: 0,
            dblclick_seek: true,
            video_autonext: true,
            played_pct: 80.0,
            volume_boost: 100.0,
            night_mode: false,
            downmix: false,
            ytdl_format: 1,
            ytdl: true,
            cache_mb: 200,
            sub_font: "Segoe UI Semibold".into(),
            sub_color: 0,
            sub_style: 0,
            sub_pos: 100,
            sub_bold: false,
            sub_override: false,
            sub_auto: 2,
            single_instance: true,
            tray: false,
            custom_keys: Vec::new(),
            podcasts: Vec::new(),
            welcome_done: false,
            media_keys: true,
            global_hotkeys: true,
            prevent_sleep: true,
            check_updates: true,
            library_folders: dirs::audio_dir().map(|p| vec![p.to_string_lossy().into_owned()]).unwrap_or_default(),
            video_folders: dirs::video_dir().map(|p| vec![p.to_string_lossy().into_owned()]).unwrap_or_default(),
            custom_eq: Vec::new(),
            nav_collapsed: Vec::new(),
            nav_hidden: Vec::new(),
            sidebar_width: 236.0,
            smart_lists: vec![
                (crate::tr!("Top rated").into(), "rating>=4".into()),
                (crate::tr!("Played often").into(), "plays>=5".into()),
            ],
            skip_version: String::new(),
            audio_rev: AUDIO_REV,
        }
    }
}

const AUDIO_REV: u32 = 1;

impl Settings {
    pub fn load() -> Settings {
        let mut s: Settings = crate::paths::load_json("settings.json");
        if s.audio_rev < AUDIO_REV {
            // Do 1.1.3 ReplayGain był domyślnie włączony i ściszał głośne nagrania o 6–10 dB –
            // muzyka grała dużo ciszej niż w innych odtwarzaczach.
            s.replaygain = 0;
            s.audio_rev = AUDIO_REV;
        }
        // Do 1.2.7 dekodowanie sprzętowe było tylko włączone/wyłączone.
        if !s.hwdec {
            s.hwdec_mode = 0;
        }
        s.hwdec = s.hwdec_mode != 0;
        s
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
