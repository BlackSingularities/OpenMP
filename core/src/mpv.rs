//! Cienka warstwa nad libmpv (ładowaną dynamicznie z `libmpv-2.dll` obok programu): odtwarzanie
//! wideo, radia internetowego i formatów audio, których nie obsługuje silnik muzyki.
//! Obraz rysujemy sami przez render API (OpenGL) w oknie Slint.

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::PathBuf;
use std::sync::Arc;

use libloading::Library;

type Handle = *mut c_void;

#[repr(C)]
struct RawEvent {
    event_id: c_int,
    error: c_int,
    reply_userdata: u64,
    data: *mut c_void,
}

#[repr(C)]
struct RawEndFile {
    reason: c_int,
    error: c_int,
}

#[repr(C)]
struct RenderParam {
    kind: c_int,
    data: *mut c_void,
}

#[repr(C)]
struct GlInitParams {
    get_proc_address: unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void,
    ctx: *mut c_void,
}

#[repr(C)]
struct GlFbo {
    fbo: c_int,
    w: c_int,
    h: c_int,
    internal_format: c_int,
}

const FORMAT_FLAG: c_int = 3;
const FORMAT_INT64: c_int = 4;
const FORMAT_DOUBLE: c_int = 5;

const RP_API_TYPE: c_int = 1;
const RP_GL_INIT: c_int = 2;
const RP_GL_FBO: c_int = 3;
const RP_FLIP_Y: c_int = 4;

struct Api {
    _lib: Library,
    create: unsafe extern "C" fn() -> Handle,
    initialize: unsafe extern "C" fn(Handle) -> c_int,
    terminate_destroy: unsafe extern "C" fn(Handle),
    set_option_string: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int,
    command: unsafe extern "C" fn(Handle, *mut *const c_char) -> c_int,
    command_async: unsafe extern "C" fn(Handle, u64, *mut *const c_char) -> c_int,
    set_property_string: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int,
    set_property: unsafe extern "C" fn(Handle, *const c_char, c_int, *mut c_void) -> c_int,
    get_property: unsafe extern "C" fn(Handle, *const c_char, c_int, *mut c_void) -> c_int,
    get_property_string: unsafe extern "C" fn(Handle, *const c_char) -> *mut c_char,
    free: unsafe extern "C" fn(*mut c_void),
    wait_event: unsafe extern "C" fn(Handle, f64) -> *mut RawEvent,
    rc_create: unsafe extern "C" fn(*mut Handle, Handle, *mut RenderParam) -> c_int,
    rc_set_update_callback: unsafe extern "C" fn(Handle, Option<unsafe extern "C" fn(*mut c_void)>, *mut c_void),
    rc_update: unsafe extern "C" fn(Handle) -> u64,
    rc_render: unsafe extern "C" fn(Handle, *mut RenderParam) -> c_int,
    rc_free: unsafe extern "C" fn(Handle),
}

unsafe impl Send for Api {}
unsafe impl Sync for Api {}

impl Api {
    fn load() -> Result<Api, String> {
        let mut tried = Vec::new();
        let mut dirs: Vec<PathBuf> = Vec::new();
        if let Some(d) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())) {
            dirs.push(d.clone());
            dirs.push(d.join("mpv"));
        }
        let mut lib = None;
        for d in dirs.iter().map(Some).chain([None]) {
            for name in ["libmpv-2.dll", "mpv-2.dll", "mpv-1.dll"] {
                let p = match d {
                    Some(d) => d.join(name),
                    None => PathBuf::from(name),
                };
                tried.push(p.display().to_string());
                if let Ok(l) = unsafe { Library::new(&p) } {
                    lib = Some(l);
                    break;
                }
            }
            if lib.is_some() {
                break;
            }
        }
        let lib = lib.ok_or_else(|| "Nie znaleziono biblioteki libmpv-2.dll obok programu".to_string())?;
        unsafe {
            macro_rules! sym {
                ($n:literal) => {
                    *lib.get(concat!($n, "\0").as_bytes()).map_err(|e| format!("libmpv: {} ({e})", $n))?
                };
            }
            Ok(Api {
                create: sym!("mpv_create"),
                initialize: sym!("mpv_initialize"),
                terminate_destroy: sym!("mpv_terminate_destroy"),
                set_option_string: sym!("mpv_set_option_string"),
                command: sym!("mpv_command"),
                command_async: sym!("mpv_command_async"),
                set_property_string: sym!("mpv_set_property_string"),
                set_property: sym!("mpv_set_property"),
                get_property: sym!("mpv_get_property"),
                get_property_string: sym!("mpv_get_property_string"),
                free: sym!("mpv_free"),
                wait_event: sym!("mpv_wait_event"),
                rc_create: sym!("mpv_render_context_create"),
                rc_set_update_callback: sym!("mpv_render_context_set_update_callback"),
                rc_update: sym!("mpv_render_context_update"),
                rc_render: sym!("mpv_render_context_render"),
                rc_free: sym!("mpv_render_context_free"),
                _lib: lib,
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    FileLoaded,
    /// Koniec pliku: `true` – dograł do końca (EOF), `false` – błąd.
    EndFile { eof: bool, error: bool },
    VideoReconfig,
    AudioReconfig,
    Seek,
    PlaybackRestart,
    Shutdown,
}

struct Inner {
    api: Api,
    h: Handle,
}

unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}

impl Drop for Inner {
    fn drop(&mut self) {
        unsafe { (self.api.terminate_destroy)(self.h) }
    }
}

#[derive(Clone)]
pub struct Mpv(Arc<Inner>);

fn cs(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_default()
}

impl Mpv {
    /// Tworzy odtwarzacz; `opts` – opcje ustawiane przed inicjalizacją.
    pub fn new(opts: &[(&str, &str)]) -> Result<Mpv, String> {
        let api = Api::load()?;
        let h = unsafe { (api.create)() };
        if h.is_null() {
            return Err("mpv_create się nie powiódł".into());
        }
        let inner = Inner { api, h };
        for (k, v) in opts {
            let (k, v) = (cs(k), cs(v));
            unsafe { (inner.api.set_option_string)(h, k.as_ptr(), v.as_ptr()) };
        }
        let r = unsafe { (inner.api.initialize)(h) };
        if r < 0 {
            return Err(format!("mpv_initialize: {r}"));
        }
        Ok(Mpv(Arc::new(inner)))
    }

    pub fn command(&self, args: &[&str]) -> Result<(), i32> {
        let owned: Vec<CString> = args.iter().map(|a| cs(a)).collect();
        let mut ptrs: Vec<*const c_char> = owned.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(std::ptr::null());
        let r = unsafe { (self.0.api.command)(self.0.h, ptrs.as_mut_ptr()) };
        if r < 0 {
            Err(r)
        } else {
            Ok(())
        }
    }

    /// Polecenie bez czekania (wczytywanie z sieci nie blokuje interfejsu).
    pub fn command_async(&self, args: &[&str]) {
        let owned: Vec<CString> = args.iter().map(|a| cs(a)).collect();
        let mut ptrs: Vec<*const c_char> = owned.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(std::ptr::null());
        unsafe { (self.0.api.command_async)(self.0.h, 0, ptrs.as_mut_ptr()) };
    }

    pub fn set(&self, name: &str, value: &str) {
        let (n, v) = (cs(name), cs(value));
        unsafe { (self.0.api.set_property_string)(self.0.h, n.as_ptr(), v.as_ptr()) };
    }

    pub fn set_f64(&self, name: &str, mut v: f64) {
        let n = cs(name);
        unsafe { (self.0.api.set_property)(self.0.h, n.as_ptr(), FORMAT_DOUBLE, &mut v as *mut f64 as *mut c_void) };
    }

    pub fn set_flag(&self, name: &str, v: bool) {
        let n = cs(name);
        let mut f: c_int = v as c_int;
        unsafe { (self.0.api.set_property)(self.0.h, n.as_ptr(), FORMAT_FLAG, &mut f as *mut c_int as *mut c_void) };
    }

    pub fn get_str(&self, name: &str) -> Option<String> {
        let n = cs(name);
        unsafe {
            let p = (self.0.api.get_property_string)(self.0.h, n.as_ptr());
            if p.is_null() {
                return None;
            }
            let s = CStr::from_ptr(p).to_string_lossy().into_owned();
            (self.0.api.free)(p as *mut c_void);
            Some(s)
        }
    }

    pub fn get_f64(&self, name: &str) -> Option<f64> {
        let n = cs(name);
        let mut v = 0f64;
        let r = unsafe { (self.0.api.get_property)(self.0.h, n.as_ptr(), FORMAT_DOUBLE, &mut v as *mut f64 as *mut c_void) };
        (r >= 0).then_some(v)
    }

    pub fn get_i64(&self, name: &str) -> Option<i64> {
        let n = cs(name);
        let mut v = 0i64;
        let r = unsafe { (self.0.api.get_property)(self.0.h, n.as_ptr(), FORMAT_INT64, &mut v as *mut i64 as *mut c_void) };
        (r >= 0).then_some(v)
    }

    pub fn get_flag(&self, name: &str) -> Option<bool> {
        let n = cs(name);
        let mut v: c_int = 0;
        let r = unsafe { (self.0.api.get_property)(self.0.h, n.as_ptr(), FORMAT_FLAG, &mut v as *mut c_int as *mut c_void) };
        (r >= 0).then_some(v != 0)
    }

    /// Zdarzenia bez czekania (wołane z zegara interfejsu).
    pub fn poll(&self) -> Vec<Event> {
        let mut out = Vec::new();
        for _ in 0..64 {
            let ev = unsafe { &*(self.0.api.wait_event)(self.0.h, 0.0) };
            match ev.event_id {
                0 => break,
                1 => out.push(Event::Shutdown),
                7 => {
                    let (reason, err) = if ev.data.is_null() {
                        (0, 0)
                    } else {
                        let e = unsafe { &*(ev.data as *const RawEndFile) };
                        (e.reason, e.error)
                    };
                    // Zatrzymanie przez nas (loadfile następnego) nie jest końcem odtwarzania.
                    if reason == 0 || reason == 4 {
                        out.push(Event::EndFile { eof: reason == 0, error: reason == 4 || err < 0 });
                    }
                }
                8 => out.push(Event::FileLoaded),
                17 => out.push(Event::VideoReconfig),
                18 => out.push(Event::AudioReconfig),
                20 => out.push(Event::Seek),
                21 => out.push(Event::PlaybackRestart),
                _ => {}
            }
        }
        out
    }
}

/// Kontekst rysowania obrazu w OpenGL okna.
pub struct Render {
    mpv: Mpv,
    ctx: Handle,
    cb: *mut Box<dyn Fn() + Send + Sync>,
}

unsafe impl Send for Render {}

unsafe extern "C" fn proc_trampoline(ctx: *mut c_void, name: *const c_char) -> *mut c_void {
    let f = &*(ctx as *const &dyn Fn(&CStr) -> *const c_void);
    f(CStr::from_ptr(name)) as *mut c_void
}

unsafe extern "C" fn update_trampoline(ctx: *mut c_void) {
    let f = &*(ctx as *const Box<dyn Fn() + Send + Sync>);
    f();
}

impl Render {
    /// `get_proc_address` jest używane tylko w trakcie tworzenia kontekstu.
    pub fn new(mpv: &Mpv, get_proc_address: &dyn Fn(&CStr) -> *const c_void, on_update: Box<dyn Fn() + Send + Sync>) -> Result<Render, String> {
        let api_type = cs("opengl");
        let mut gp: &dyn Fn(&CStr) -> *const c_void = get_proc_address;
        let mut init = GlInitParams { get_proc_address: proc_trampoline, ctx: &mut gp as *mut _ as *mut c_void };
        let mut params = [
            RenderParam { kind: RP_API_TYPE, data: api_type.as_ptr() as *mut c_void },
            RenderParam { kind: RP_GL_INIT, data: &mut init as *mut _ as *mut c_void },
            RenderParam { kind: 0, data: std::ptr::null_mut() },
        ];
        let mut ctx: Handle = std::ptr::null_mut();
        let r = unsafe { (mpv.0.api.rc_create)(&mut ctx, mpv.0.h, params.as_mut_ptr()) };
        if r < 0 || ctx.is_null() {
            return Err(format!("mpv_render_context_create: {r}"));
        }
        let cb = Box::into_raw(Box::new(on_update));
        unsafe { (mpv.0.api.rc_set_update_callback)(ctx, Some(update_trampoline), cb as *mut c_void) };
        Ok(Render { mpv: mpv.clone(), ctx, cb })
    }

    /// Czy jest nowa klatka do narysowania.
    pub fn needs_frame(&self) -> bool {
        unsafe { (self.mpv.0.api.rc_update)(self.ctx) & 1 != 0 }
    }

    /// Rysuje bieżącą klatkę do bufora ramki `fbo` o rozmiarze `w`×`h`.
    pub fn draw(&self, fbo: i32, w: i32, h: i32) {
        let mut f = GlFbo { fbo, w, h, internal_format: 0 };
        let mut flip: c_int = 0;
        let mut params = [
            RenderParam { kind: RP_GL_FBO, data: &mut f as *mut _ as *mut c_void },
            RenderParam { kind: RP_FLIP_Y, data: &mut flip as *mut _ as *mut c_void },
            RenderParam { kind: 0, data: std::ptr::null_mut() },
        ];
        unsafe { (self.mpv.0.api.rc_render)(self.ctx, params.as_mut_ptr()) };
    }
}

impl Drop for Render {
    fn drop(&mut self) {
        unsafe {
            (self.mpv.0.api.rc_set_update_callback)(self.ctx, None, std::ptr::null_mut());
            (self.mpv.0.api.rc_free)(self.ctx);
            drop(Box::from_raw(self.cb));
        }
    }
}

/// Ścieżka (dźwiękowa, napisów, wideo) z `track-list`.
#[derive(Clone, Debug, Default)]
pub struct TrackEntry {
    pub id: i64,
    pub kind: String,
    pub title: String,
    pub lang: String,
    pub codec: String,
    pub selected: bool,
    pub external: bool,
}

impl Mpv {
    pub fn tracks(&self) -> Vec<TrackEntry> {
        let Some(js) = self.get_str("track-list") else { return Vec::new() };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&js) else { return Vec::new() };
        v.as_array()
            .map(|a| {
                a.iter()
                    .map(|t| TrackEntry {
                        id: t["id"].as_i64().unwrap_or(0),
                        kind: t["type"].as_str().unwrap_or("").into(),
                        title: t["title"].as_str().unwrap_or("").into(),
                        lang: t["lang"].as_str().unwrap_or("").into(),
                        codec: t["codec"].as_str().unwrap_or("").into(),
                        selected: t["selected"].as_bool().unwrap_or(false),
                        external: t["external"].as_bool().unwrap_or(false),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Rozdziały: (czas, tytuł).
    pub fn chapters(&self) -> Vec<(f64, String)> {
        let Some(js) = self.get_str("chapter-list") else { return Vec::new() };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&js) else { return Vec::new() };
        v.as_array()
            .map(|a| a.iter().map(|c| (c["time"].as_f64().unwrap_or(0.0), c["title"].as_str().unwrap_or("").to_string())).collect())
            .unwrap_or_default()
    }
}
