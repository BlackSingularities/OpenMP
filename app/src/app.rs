//! Stan programu żyjący w wątku interfejsu (thread_local) i modele list dla Slint.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Instant;

use openmp_core::audio::engine::Engine;
use openmp_core::audio::viz::Analyzer;
use openmp_core::library::{Library, Stats};
use openmp_core::meta::TrackInfo;
use openmp_core::mpv::Mpv;
use openmp_core::playlist::Store;
use openmp_core::radio::Station;
use openmp_core::settings::{Session, Settings};
use slint::{ModelRc, SharedString, VecModel};

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Backend {
    Engine,
    Mpv,
}

/// Co teraz gra.
#[derive(Clone, Debug)]
pub struct Playing {
    /// Lista i pozycja (radio i pliki spoza list – `None`).
    pub list: Option<u64>,
    pub idx: usize,
    pub item: u64,
    pub info: TrackInfo,
    pub backend: Backend,
    pub video: bool,
    pub station: Option<Station>,
}

/// Zapowiedziany następny utwór (przejście bez przerwy w silniku muzyki).
#[derive(Clone, Debug)]
pub struct Pending {
    pub item: u64,
    pub list: u64,
    pub idx: usize,
    pub from_queue: bool,
}

/// Do czego odnosi się otwarte menu.
#[derive(Clone, Debug, Default)]
pub enum MenuCtx {
    #[default]
    None,
    Main,
    List,
    Lib,
    Tab(usize),
    Tile(String),
    Nav(String),
    Sleep,
    Video,
    Device,
}

#[derive(Clone, Debug, Default)]
pub enum RenameCtx {
    #[default]
    None,
    Tab(usize),
    EqPreset,
    /// Ręczny tytuł do wyszukania w TMDB dla pliku.
    Tmdb(String),
    RadioUrl,
    /// Ścieżka sieciowa: 0 – muzyka, 1 – wideo.
    NetFolder(i32),
    /// Inteligentna lista: nazwa albo zapytanie.
    SmartName(usize),
    SmartQuery(usize),
    /// Adres kanału RSS nowego podcastu.
    Podcast,
}

#[derive(Clone, Debug)]
pub enum SleepMode {
    At(Instant),
    EndOfTrack,
    EndOfList,
}

/// Widok biblioteki: rodzaj (wykonawcy, albumy…) i stos wejść w głąb.
#[derive(Clone, Debug, Default)]
pub struct LibState {
    pub kind: String,
    pub stack: Vec<(String, String, String)>,
    pub tracks: Vec<TrackInfo>,
    pub sel: HashSet<usize>,
    pub anchor: Option<usize>,
    pub tile_keys: Vec<(String, String, String)>,
    /// Rozwinięte foldery w panelu nawigacji (ścieżki małymi literami).
    pub expanded: HashSet<String>,
    pub search: String,
}

pub struct Models {
    pub bars: Rc<VecModel<f32>>,
    pub peaks: Rc<VecModel<f32>>,
    pub rows: Rc<VecModel<TrackRow>>,
    pub lib_rows: Rc<VecModel<TrackRow>>,
    pub tiles: Rc<VecModel<Tile>>,
    pub nav: Rc<VecModel<NavItem>>,
    pub tabs: Rc<VecModel<TabItem>>,
    pub eq: Rc<VecModel<f32>>,
    pub lyrics: Rc<VecModel<LyricLine>>,
    pub stations: Rc<VecModel<Station_>>,
}

pub type Station_ = crate::Station;

pub struct App {
    pub ui: slint::Weak<AppWindow>,
    pub s: Settings,
    pub session: Session,
    pub store: Store,
    pub lib: Library,
    pub stats: Stats,
    /// Pomiary głośności R128 plików bez ReplayGain i pliki mierzone teraz.
    pub loudness: openmp_core::audio::loudness::Store,
    pub measuring: std::collections::HashSet<String>,
    pub engine: Engine,
    pub mpv: Option<Mpv>,
    pub mpv_err: String,
    pub m: Models,
    pub cur: Option<Playing>,
    pub pending: Option<Pending>,
    pub seq: u64,
    pub queue: Vec<(u64, usize)>,
    pub history: Vec<(u64, usize)>,
    pub shuffle: Vec<usize>,
    pub shuffle_list: u64,
    pub shuffle_len: usize,
    // Lista na ekranie
    pub sel: HashSet<usize>,
    pub anchor: Option<usize>,
    pub row_of: HashMap<usize, usize>,
    pub filter: String,
    pub libv: LibState,
    pub analyzer: Analyzer,
    pub last_frame: Instant,
    /// Ostatnie doliczenie czasu słuchania do statystyk.
    pub listen_at: Instant,
    pub menu: MenuCtx,
    /// Okno menu kontekstowego (tworzone przy pierwszym użyciu).
    pub menu_win: Option<MenuWindow>,
    /// Okno dialogów (ustawienia, korektor…).
    pub dialogs: Option<crate::dialogwin::Dialogs>,
    pub rename: RenameCtx,
    pub sleep: Option<SleepMode>,
    pub sleep_fade: Option<f32>,
    pub toast_until: Option<Instant>,
    pub osd_until: Option<Instant>,
    pub last_mouse: Instant,
    pub hover_controls: bool,
    /// Odliczanie do następnego odcinka anulowane dla bieżącego filmu.
    pub next_cancelled: bool,
    pub ab: (Option<f64>, Option<f64>),
    pub lyrics: Vec<(f64, String)>,
    pub lyrics_text: String,
    /// Tryb zaznaczenia: plik, którego opis i tekst są pokazane (pusty – brak) i opis z TMDB.
    pub sel_text: String,
    pub sel_desc: Option<openmp_core::tmdb::Info>,
    /// Napisy filmu w panelu tekstu: ścieżki tekstowe i wybrana (id mpv, 0 – brak).
    pub sub_panel: Vec<openmp_core::mpv::TrackEntry>,
    pub sub_panel_id: i64,
    /// Napisy z kontenera – zbierane na bieżąco z mpv (nie da się ich wczytać z góry).
    pub sub_live: bool,
    /// Wiersze tekstu / napisów pasujące do wyszukiwania w panelu.
    pub lyrics_hits: Vec<usize>,
    pub wave_raw: Vec<f32>,
    /// Ostatnie widma do widma 3D.
    pub viz_hist: std::collections::VecDeque<Vec<f32>>,
    pub music_geom: Option<(i32, i32, u32, u32)>,
    pub dirty: bool,
    /// Element (`Playing::item`) już wysłany do ListenBrainz.
    pub scrobbled: u64,
    /// Odtworzenie (id), które już zostało policzone jako odsłuchane / obejrzane.
    pub counted: u64,
    /// Plik, którego duża okładka jest (albo będzie) w zaznaczeniu.
    pub sel_cover_path: String,
    pub last_save: Instant,
    pub last_smtc: Instant,
    pub stations: Vec<Station>,
    /// Odcinki pobranych kanałów podcastów (adres kanału → odcinki), na czas działania programu.
    pub podcasts: std::collections::HashMap<String, Vec<TrackInfo>>,
    pub saved_stations: Vec<Station>,
    pub update: Option<openmp_core::update::Release>,
    pub cover_bytes: Option<Vec<u8>>,
    pub tag_paths: Vec<String>,
    pub tag_mixed: Vec<bool>,
    pub tag_cover: Option<Vec<u8>>,
    pub smtc: Option<souvlaki::MediaControls>,
    pub scan: Option<std::sync::Arc<openmp_core::library::Progress>>,
    /// Obserwator folderów biblioteki i czas ostatniej zmiany na dysku (ms od epoki, 0 – brak).
    pub watch: Option<(notify::RecommendedWatcher, std::sync::Arc<std::sync::atomic::AtomicU64>)>,
    pub video_size: (i64, i64),
    /// Rozdziały bieżącego filmu (czas, tytuł).
    pub chapters: Vec<(f64, String)>,
    /// Niewidoczny odtwarzacz podglądu klatek, wczytany w nim plik, żądany i wysłany czas.
    pub preview: Option<Mpv>,
    pub preview_file: String,
    pub preview_want: Option<f64>,
    pub preview_sent: f64,
    pub preview_at: Instant,
    /// Barwa obrazu, do której płynnie dąży kolor przewodni.
    pub tint_target: Option<slint::Color>,
    pub video_slow: Instant,
    pub awake: bool,
    pub icy: String,
    pub pause_after_load: bool,
    pub last_pos: f64,
    pub last_dur: f64,
}

thread_local! {
    pub static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Dostęp do stanu. Wywołanie zagnieżdżone (np. z callbacku wywołanego przez zmianę właściwości)
/// zostaje odłożone na następny obieg pętli zdarzeń.
pub fn with<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|a| {
        let mut b = a.try_borrow_mut().ok()?;
        let app = b.as_mut()?;
        Some(f(app))
    })
}

/// Jak `with`, ale gdy stan jest zajęty – wykonanie po bieżącym zdarzeniu.
pub fn later(f: impl FnOnce(&mut App) + 'static) {
    let busy = APP.with(|a| a.try_borrow_mut().is_err());
    if busy {
        slint::Timer::single_shot(std::time::Duration::ZERO, move || {
            with(f);
        });
    } else {
        with(f);
    }
}

pub fn ss(s: impl AsRef<str>) -> SharedString {
    SharedString::from(s.as_ref())
}

pub fn model<T: Clone + 'static>(v: &Rc<VecModel<T>>) -> ModelRc<T> {
    ModelRc::from(v.clone())
}

pub fn strings(v: &[String]) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(v.iter().map(ss).collect::<Vec<_>>()))
}

impl App {
    pub fn ui(&self) -> Option<AppWindow> {
        self.ui.upgrade()
    }

    pub fn toast(&mut self, text: impl AsRef<str>) {
        if let Some(ui) = self.ui() {
            ui.global::<Ui>().set_toast(ss(text));
            ui.global::<Ui>().set_toast_visible(true);
        }
        self.toast_until = Some(Instant::now() + std::time::Duration::from_millis(2600));
    }

    pub fn osd(&mut self, text: impl AsRef<str>) {
        let t = text.as_ref();
        let (icon, level) = osd_style(t);
        if let Some(ui) = self.ui() {
            let v = ui.global::<Video>();
            v.set_osd(ss(t));
            v.set_osd_icon(ss(icon));
            v.set_osd_level(level);
            v.set_osd_visible(true);
        }
        self.osd_until = Some(Instant::now() + std::time::Duration::from_millis(1300));
    }

    pub fn list_index(&self, id: u64) -> Option<usize> {
        self.store.lists.iter().position(|l| l.id == id)
    }

    pub fn active_id(&self) -> u64 {
        self.store.lists.get(self.store.active).map(|l| l.id).unwrap_or(0)
    }

    pub fn next_seq(&mut self) -> u64 {
        self.seq += 1;
        self.seq
    }

    pub fn is_video_mode(&self) -> bool {
        self.ui().map(|u| u.global::<Ui>().get_mode() == 1).unwrap_or(false)
    }
}

/// Ikona i wartość paska dla komunikatu na obrazie – z jego treści.
fn osd_style(t: &str) -> (&'static str, f32) {
    // Liczba przed „%” albo „×” w tekście.
    let num = |end: char| -> Option<f32> {
        let i = t.find(end)?;
        let head = &t[..i];
        let start = head.rfind(|c: char| !(c.is_ascii_digit() || c == '.' || c == ',')).map(|k| k + 1).unwrap_or(0);
        head[start..].replace(',', ".").parse().ok()
    };
    // Rodzaj komunikatu po początku tekstu – przetłumaczonym tak samo jak przy wywołaniu osd().
    let is = |ids: &[&'static str]| {
        ids.iter().any(|id| {
            let s = openmp_core::i18n::tr(*id);
            let head = s.split('{').next().unwrap_or(s).trim_end();
            !head.is_empty() && t.starts_with(head)
        })
    };
    if is(&["Volume {}%"]) {
        return ("volume", num('%').map(|v| v / 100.0).unwrap_or(-1.0));
    }
    if is(&["Muted"]) {
        return ("volume-off", -1.0);
    }
    if is(&["Sound on"]) {
        return ("volume", -1.0);
    }
    if is(&["Speed {}×"]) {
        return ("speed", num('×').map(|v| (v - 0.25) / 3.75).unwrap_or(-1.0));
    }
    if is(&["Zoom {}%"]) {
        return ("expand", num('%').map(|v| (v / 800.0).sqrt()).unwrap_or(-1.0));
    }
    let icon = if is(&["Always on top", "Normal window"]) {
        "pin"
    } else if is(&["Subtitles: {} ms", "Subtitles", "Subtitles downloaded"]) {
        "subtitles"
    } else if is(&["Audio: {} ms", "Audio track"]) {
        "audiotrack"
    } else if is(&["Rotation {}°", "Aspect ratio: {}", "Picture as in file"]) {
        "aspect"
    } else if is(&["Screenshot saved", "Screenshot (without subtitles) saved"]) {
        "camera"
    } else if is(&["Resumed from {}  ·  Home – from the start"]) {
        "history"
    } else if t.starts_with('+') || t.starts_with('−') || t.starts_with('-') {
        "forward"
    } else if is(&["Ambient glow on", "Ambient glow off", "Debanding: on", "Debanding: off"]) {
        "tune"
    } else {
        "info"
    };
    (icon, -1.0)
}
