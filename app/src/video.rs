//! Tryb wideo: mpv rysuje klatki do tekstury OpenGL okna Slint, panel sterowania chowa się
//! po chwili bez ruchu myszy, okno dopasowuje się do filmu.

use std::cell::RefCell;
use std::num::NonZeroU32;
use std::time::Instant;

use glow::HasContext;
use openmp_core::meta;
use openmp_core::mpv::{Mpv, Render};
use slint::{ComponentHandle, GraphicsAPI, ModelRc, RenderingState, VecModel};

use crate::app::*;
use crate::*;

struct Gl {
    gl: glow::Context,
    render: Render,
    fbo: Option<glow::Framebuffer>,
    tex: Option<glow::Texture>,
    size: (u32, u32),
}

thread_local! {
    static GL: RefCell<Option<Gl>> = const { RefCell::new(None) };
}

/// Opcje mpv zgodne z ustawieniami programu.
pub fn create(s: &openmp_core::settings::Settings) -> Result<Mpv, String> {
    let shots = s.screenshot_dir.clone();
    let _ = std::fs::create_dir_all(&shots);
    let sub_size = s.sub_size.to_string();
    let opts: Vec<(&str, &str)> = vec![
        ("vo", "libmpv"),
        ("hwdec", if s.hwdec { "auto-safe" } else { "no" }),
        ("idle", "yes"),
        ("keep-open", "no"),
        ("config", "no"),
        ("terminal", "no"),
        ("input-default-bindings", "no"),
        ("input-vo-keyboard", "no"),
        ("osc", "no"),
        ("osd-level", "0"),
        ("audio-display", "no"),
        ("ytdl", "yes"),
        ("sub-auto", "fuzzy"),
        ("sub-font-size", &sub_size),
        ("sub-border-size", "2.5"),
        ("sub-shadow-offset", "1"),
        ("slang", &s.sub_lang),
        ("alang", &s.audio_lang),
        ("screenshot-directory", &shots),
        ("screenshot-format", "png"),
        ("screenshot-template", "OpenMP-%F-%P"),
        ("cache", "yes"),
        ("demuxer-max-bytes", "200MiB"),
        ("network-timeout", "20"),
        ("user-agent", "OpenMP"),
        ("deinterlace", if s.deinterlace { "yes" } else { "no" }),
        ("volume-max", "100"),
        ("title", "OpenMP"),
    ];
    Mpv::new(&opts)
}

/// Podpina rysowanie mpv do okna.
pub fn attach(ui: &AppWindow, mpv: Mpv) {
    let weak = ui.as_weak();
    let res = ui.window().set_rendering_notifier(move |state, api| match state {
        RenderingState::RenderingSetup => {
            if let GraphicsAPI::NativeOpenGL { get_proc_address } = api {
                let gl = unsafe { glow::Context::from_loader_function_cstr(|s| get_proc_address(s)) };
                let w = weak.clone();
                let redraw: Box<dyn Fn() + Send + Sync> = Box::new(move || {
                    let w = w.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = w.upgrade() {
                            ui.window().request_redraw();
                        }
                    });
                });
                match Render::new(&mpv, get_proc_address, redraw) {
                    Ok(render) => GL.with(|g| *g.borrow_mut() = Some(Gl { gl, render, fbo: None, tex: None, size: (0, 0) })),
                    Err(e) => eprintln!("mpv render: {e}"),
                }
            }
        }
        RenderingState::BeforeRendering => {
            if let Some(ui) = weak.upgrade() {
                draw(&ui);
            }
        }
        RenderingState::RenderingTeardown => {
            GL.with(|g| {
                if let Some(g) = g.borrow_mut().take() {
                    unsafe {
                        if let Some(f) = g.fbo {
                            g.gl.delete_framebuffer(f);
                        }
                        if let Some(t) = g.tex {
                            g.gl.delete_texture(t);
                        }
                    }
                }
            });
        }
        _ => {}
    });
    if let Err(e) = res {
        eprintln!("rendering notifier: {e:?}");
    }
}

fn draw(ui: &AppWindow) {
    if ui.global::<Ui>().get_mode() != 1 {
        return;
    }
    let size = ui.window().size();
    let (w, h) = (size.width.max(1), size.height.max(1));
    GL.with(|g| {
        let mut g = g.borrow_mut();
        let Some(g) = g.as_mut() else { return };
        let gl = &g.gl;
        unsafe {
            let prev_fbo = gl.get_parameter_i32(glow::FRAMEBUFFER_BINDING);
            let mut vp = [0i32; 4];
            gl.get_parameter_i32_slice(glow::VIEWPORT, &mut vp);
            let prev_tex = gl.get_parameter_i32(glow::TEXTURE_BINDING_2D);
            let prev_active = gl.get_parameter_i32(glow::ACTIVE_TEXTURE);
            let prev_prog = gl.get_parameter_i32(glow::CURRENT_PROGRAM);
            let prev_vao = gl.get_parameter_i32(glow::VERTEX_ARRAY_BINDING);
            let prev_buf = gl.get_parameter_i32(glow::ARRAY_BUFFER_BINDING);
            let blend = gl.is_enabled(glow::BLEND);
            let scissor = gl.is_enabled(glow::SCISSOR_TEST);
            let restore_fbo = NonZeroU32::new(prev_fbo as u32).map(glow::NativeFramebuffer);
            let mut force = false;
            if g.tex.is_none() || g.size != (w, h) {
                if let Some(f) = g.fbo.take() {
                    gl.delete_framebuffer(f);
                }
                if let Some(t) = g.tex.take() {
                    gl.delete_texture(t);
                }
                let Ok(tex) = gl.create_texture() else { return };
                gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, w as i32, h as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(None));
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
                let Ok(fbo) = gl.create_framebuffer() else { return };
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
                gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(tex), 0);
                gl.clear_color(0.0, 0.0, 0.0, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT);
                g.tex = Some(tex);
                g.fbo = Some(fbo);
                g.size = (w, h);
                force = true;
                let img = slint::BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(tex.0, (w, h).into())
                    .origin(slint::BorrowedOpenGLTextureOrigin::TopLeft)
                    .build();
                ui.global::<Video>().set_frame(img);
                ui.global::<Video>().set_has_frame(true);
            }
            if g.render.needs_frame() || force {
                if let Some(fbo) = g.fbo {
                    g.render.draw(fbo.0.get() as i32, w as i32, h as i32);
                }
            }
            // Przywrócenie stanu, którego używa renderer Slint.
            gl.bind_framebuffer(glow::FRAMEBUFFER, restore_fbo);
            gl.viewport(vp[0], vp[1], vp[2], vp[3]);
            gl.active_texture(prev_active as u32);
            gl.bind_texture(glow::TEXTURE_2D, NonZeroU32::new(prev_tex as u32).map(glow::NativeTexture));
            gl.use_program(NonZeroU32::new(prev_prog as u32).map(glow::NativeProgram));
            gl.bind_vertex_array(NonZeroU32::new(prev_vao as u32).map(glow::NativeVertexArray));
            gl.bind_buffer(glow::ARRAY_BUFFER, NonZeroU32::new(prev_buf as u32).map(glow::NativeBuffer));
            if blend {
                gl.enable(glow::BLEND);
            } else {
                gl.disable(glow::BLEND);
            }
            if scissor {
                gl.enable(glow::SCISSOR_TEST);
            } else {
                gl.disable(glow::SCISSOR_TEST);
            }
        }
    });
}

pub fn enter_video(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    let u = ui.global::<Ui>();
    if u.get_mode() != 1 {
        if u.get_mode() == 0 && !ui.window().is_maximized() {
            app.music_geom = Some(crate::win::geometry(&ui));
        }
        u.set_mode(1);
        crate::win::set_ontop(app, ui.global::<Video>().get_ontop());
    }
    let v = ui.global::<Video>();
    v.set_controls(true);
    v.set_error(ss(""));
    app.last_mouse = Instant::now();
    ui.window().request_redraw();
}

pub fn leave_video(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    let u = ui.global::<Ui>();
    if u.get_mode() != 1 {
        return;
    }
    let v = ui.global::<Video>();
    if v.get_fullscreen() {
        ui.window().set_fullscreen(false);
        v.set_fullscreen(false);
    }
    v.set_has_frame(false);
    v.set_panel(false);
    u.set_mode(0);
    crate::win::set_ontop(app, app.s.always_on_top);
    if let Some(g) = app.music_geom.take() {
        crate::win::set_geometry(&ui, g);
    }
}

pub fn toggle_fullscreen(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    let v = ui.global::<Video>();
    let fs = !v.get_fullscreen();
    ui.window().set_fullscreen(fs);
    v.set_fullscreen(fs);
    app.last_mouse = Instant::now();
}

/// Plik wczytany: ścieżki, napisy, rozdziały, opis.
pub fn on_loaded(app: &mut App) {
    let Some(m) = app.mpv.clone() else { return };
    let Some(ui) = app.ui() else { return };
    let v = ui.global::<Video>();
    v.set_loading(false);
    let tracks = m.tracks();
    let has_video = tracks.iter().any(|t| t.kind == "video" && !t.title.contains("cover"));
    let is_video = app.cur.as_ref().is_some_and(|c| c.video);
    v.set_audio_only(is_video && !has_video);
    let mut audio: Vec<Opt> = Vec::new();
    let mut subs: Vec<Opt> = vec![Opt { id: ss("no"), label: ss("Wyłączone"), sub: ss(""), selected: !tracks.iter().any(|t| t.kind == "sub" && t.selected) }];
    for t in &tracks {
        let mut label = if t.title.is_empty() { format!("Ścieżka {}", t.id) } else { t.title.clone() };
        if !t.lang.is_empty() {
            label = format!("{label} [{}]", t.lang);
        }
        let mut sub = t.codec.to_uppercase();
        if t.external {
            sub.push_str(" · plik zewnętrzny");
        }
        let o = Opt { id: ss(t.id.to_string()), label: ss(label), sub: ss(sub), selected: t.selected };
        match t.kind.as_str() {
            "audio" => audio.push(o),
            "sub" => subs.push(o),
            _ => {}
        }
    }
    v.set_audio_tracks(ModelRc::new(VecModel::from(audio)));
    v.set_sub_tracks(ModelRc::new(VecModel::from(subs)));
    let dur = m.get_f64("duration").unwrap_or(0.0);
    let chapters = m.chapters();
    v.set_chapters(ModelRc::new(VecModel::from(if dur > 0.0 { chapters.iter().map(|c| (c.0 / dur) as f32).collect::<Vec<_>>() } else { Vec::new() })));
    v.set_chapter_list(ModelRc::new(VecModel::from(
        chapters
            .iter()
            .enumerate()
            .map(|(i, (t, n))| Opt { id: ss(i.to_string()), label: ss(if n.is_empty() { format!("Rozdział {}", i + 1) } else { n.clone() }), sub: ss(meta::fmt_time(*t)), selected: false })
            .collect::<Vec<_>>(),
    )));
    if let Some(title) = m.get_str("media-title") {
        if app.cur.as_ref().is_some_and(|c| c.station.is_none() && c.info.title.is_empty()) && !title.is_empty() && !title.contains("://") {
            v.set_title(ss(&title));
            if !is_video {
                ui.global::<Player>().set_title(ss(&title));
            }
        }
    }
    info_line(app);
    v.set_speed(m.get_f64("speed").unwrap_or(1.0) as f32);
    v.set_sub_delay(0.0);
    v.set_audio_delay(0.0);
    m.set_f64("sub-delay", 0.0);
    m.set_f64("audio-delay", 0.0);
    crate::player::apply_volume(app);
    if let Some(c) = &mut app.cur {
        if c.info.duration <= 0.0 && dur > 0.0 {
            c.info.duration = dur;
        }
    }
}

fn info_line(app: &mut App) {
    let Some(m) = &app.mpv else { return };
    let mut parts = Vec::new();
    if let (Some(w), Some(h)) = (m.get_i64("width"), m.get_i64("height")) {
        if w > 0 {
            parts.push(format!("{w}×{h}"));
        }
    }
    if let Some(c) = m.get_str("video-codec").filter(|c| !c.is_empty()) {
        parts.push(c.split(' ').next().unwrap_or("").to_uppercase());
    }
    if let Some(f) = m.get_f64("container-fps").filter(|f| *f > 0.0) {
        parts.push(format!("{:.3} kl./s", f).replace(".000", "").replace('.', ","));
    }
    if let Some(hw) = m.get_str("hwdec-current").filter(|h| !h.is_empty() && h != "no") {
        parts.push(format!("sprzętowo ({hw})"));
    }
    if let Some(ui) = app.ui() {
        ui.global::<Video>().set_info(ss(parts.join("  ·  ")));
    }
}

/// Zmiana rozmiaru obrazu: okno dopasowuje się do filmu (najwyżej 80% ekranu).
pub fn on_reconfig(app: &mut App) {
    info_line(app);
    let Some(m) = app.mpv.clone() else { return };
    let (Some(w), Some(h)) = (m.get_i64("dwidth"), m.get_i64("dheight")) else { return };
    if w <= 0 || h <= 0 || (w, h) == app.video_size {
        return;
    }
    app.video_size = (w, h);
    let Some(ui) = app.ui() else { return };
    if !app.s.video_fit_window || ui.window().is_maximized() || ui.global::<Video>().get_fullscreen() || ui.global::<Ui>().get_mode() != 1 {
        return;
    }
    crate::win::fit_to(&ui, w as f64, h as f64);
}

/// Filtry dźwięku mpv: korektor (firequalizer) i przedwzmocnienie.
pub fn apply_audio_filters(app: &App) {
    let Some(m) = &app.mpv else { return };
    let d = &app.s.dsp;
    if !d.eq_on {
        m.set("af", "");
        return;
    }
    let entries: Vec<String> = openmp_core::audio::dsp::BANDS.iter().zip(d.gains.iter()).map(|(f, g)| format!("entry({f},{g:.1})")).collect();
    let mut af = format!("lavfi=[firequalizer=gain_entry='{}']", entries.join(";"));
    if d.preamp.abs() > 0.05 {
        af.push_str(&format!(",lavfi=[volume={:.1}dB]", d.preamp));
    }
    m.set("af", &af);
}

pub fn command(app: &mut App, id: &str) {
    let Some(ui) = app.ui() else { return };
    let v = ui.global::<Video>();
    match id {
        "v-pause" => crate::player::play_pause(app),
        "v-full" => toggle_fullscreen(app),
        "v-ontop" => toggle_ontop(app),
        "v-audio" => open_panel(app, 0),
        "v-subs" => open_panel(app, 1),
        "v-image" => open_panel(app, 2),
        "v-chapters" => open_panel(app, 3),
        "v-shot" => screenshot(app),
        "v-ab" => ab_loop(app),
        "v-back" => back_to_music(app),
        "open-files" => crate::dialogs::open_files(app),
        _ => {}
    }
    let _ = v;
}

pub fn open_panel(app: &mut App, page: i32) {
    if let Some(ui) = app.ui() {
        ui.global::<Video>().set_panel_page(page);
        ui.global::<Video>().set_panel(true);
    }
}

pub fn toggle_ontop(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    let on = !ui.global::<Video>().get_ontop();
    ui.global::<Video>().set_ontop(on);
    crate::win::set_ontop(app, on);
    app.osd(if on { "Zawsze na wierzchu" } else { "Zwykłe okno" });
}

pub fn back_to_music(app: &mut App) {
    crate::player::set_paused(app, true);
    leave_video(app);
}

pub fn screenshot(app: &mut App) {
    if let Some(m) = &app.mpv {
        let _ = std::fs::create_dir_all(&app.s.screenshot_dir);
        m.set("screenshot-directory", &app.s.screenshot_dir);
        m.command_async(&["screenshot", "subtitles"]);
    }
    app.osd("Zrzut ekranu zapisany");
}

pub fn ab_loop(app: &mut App) {
    let (pos, dur) = crate::player::position(app);
    app.ab = match app.ab {
        (None, _) => (Some(pos), None),
        (Some(a), None) if pos > a => (Some(a), Some(pos)),
        _ => (None, None),
    };
    let msg = match app.ab {
        (Some(a), None) => format!("Pętla A-B: początek {}", meta::fmt_time(a)),
        (Some(a), Some(b)) => format!("Pętla A-B: {} – {}", meta::fmt_time(a), meta::fmt_time(b)),
        _ => "Pętla A-B wyłączona".into(),
    };
    if let Some(ui) = app.ui() {
        let p = ui.global::<Player>();
        p.set_ab_a(app.ab.0.filter(|_| dur > 0.0).map(|a| (a / dur) as f32).unwrap_or(-1.0));
        p.set_ab_b(app.ab.1.filter(|_| dur > 0.0).map(|b| (b / dur) as f32).unwrap_or(-1.0));
    }
    if app.is_video_mode() {
        app.osd(msg);
    } else {
        app.toast(msg);
    }
}

pub fn select(app: &mut App, kind: &str, id: &str) {
    let Some(m) = &app.mpv else { return };
    m.set(kind, id);
    on_loaded(app);
}

pub fn set(app: &mut App, what: &str, v: f32) {
    let Some(m) = app.mpv.clone() else { return };
    let Some(ui) = app.ui() else { return };
    let vg = ui.global::<Video>();
    match what {
        "speed" => {
            let v = (v * 20.0).round() / 20.0;
            m.set_f64("speed", v as f64);
            vg.set_speed(v);
            app.osd(format!("Prędkość {}×", format!("{v:.2}").trim_end_matches('0').trim_end_matches('.')));
        }
        "sub-delay" => {
            m.set_f64("sub-delay", v as f64);
            vg.set_sub_delay(v);
            app.osd(format!("Napisy: {:+} ms", (v * 1000.0).round()));
        }
        "audio-delay" => {
            m.set_f64("audio-delay", v as f64);
            vg.set_audio_delay(v);
            app.osd(format!("Dźwięk: {:+} ms", (v * 1000.0).round()));
        }
        "sub-scale" => {
            m.set_f64("sub-scale", v as f64);
            vg.set_sub_scale(v);
        }
        "brightness" | "contrast" | "saturation" | "gamma" => {
            m.set_f64(what, v.round() as f64);
            match what {
                "brightness" => vg.set_brightness(v.round()),
                "contrast" => vg.set_contrast(v.round()),
                "saturation" => vg.set_saturation(v.round()),
                _ => vg.set_gamma(v.round()),
            }
        }
        _ => {}
    }
}

pub fn set_aspect(app: &mut App, i: i32) {
    let Some(m) = &app.mpv else { return };
    match i {
        1 => m.set("video-aspect-override", "16:9"),
        2 => m.set("video-aspect-override", "4:3"),
        3 => m.set("video-aspect-override", "2.39:1"),
        _ => m.set("video-aspect-override", "-1"),
    }
    m.set("keepaspect", if i == 4 { "no" } else { "yes" });
    if let Some(ui) = app.ui() {
        ui.global::<Video>().set_aspect(i);
    }
}

pub fn cycle_track(app: &mut App, kind: &str) {
    let Some(m) = &app.mpv else { return };
    m.command_async(&["cycle", kind]);
    let is_sub = kind == "sid";
    let label = if is_sub { "Napisy" } else { "Ścieżka dźwiękowa" };
    let m2 = m.clone();
    slint::Timer::single_shot(std::time::Duration::from_millis(150), move || {
        with(|app| {
            on_loaded(app);
            let cur = m2.tracks().into_iter().find(|t| t.selected && t.kind == if is_sub { "sub" } else { "audio" });
            let text = match cur {
                Some(t) => format!("{label}: {}", if t.title.is_empty() { if t.lang.is_empty() { t.id.to_string() } else { t.lang } } else { t.title }),
                None => format!("{label}: wyłączone"),
            };
            app.osd(text);
        });
    });
}

pub fn chapter(app: &mut App, d: i64) {
    if let Some(m) = &app.mpv {
        m.command_async(&["add", "chapter", &d.to_string()]);
    }
}

pub fn goto_chapter(app: &mut App, id: &str) {
    if let Some(m) = &app.mpv {
        m.set("chapter", id);
    }
}

pub fn load_sub(app: &mut App) {
    let dir = app.cur.as_ref().and_then(|c| std::path::Path::new(&c.info.path).parent().map(|p| p.to_path_buf()));
    let mut d = rfd::FileDialog::new().set_title("Wczytaj napisy").add_filter("Napisy", openmp_core::meta::SUBTITLES);
    if let Some(dir) = dir {
        d = d.set_directory(dir);
    }
    if let Some(p) = d.pick_file() {
        if let Some(m) = &app.mpv {
            let _ = m.command(&["sub-add", &p.to_string_lossy(), "select"]);
        }
        on_loaded(app);
    }
}

pub fn frame_step(app: &mut App, back: bool) {
    if let Some(m) = &app.mpv {
        m.command_async(&[if back { "frame-back-step" } else { "frame-step" }]);
    }
}

/// Chowanie panelu sterowania i komunikatów.
pub fn tick(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    if let Some(t) = app.osd_until {
        if Instant::now() > t {
            ui.global::<Video>().set_osd_visible(false);
            app.osd_until = None;
        }
    }
    if ui.global::<Ui>().get_mode() != 1 {
        return;
    }
    let v = ui.global::<Video>();
    let idle = app.last_mouse.elapsed().as_millis() as i32 > app.s.autohide_ms;
    let show = !idle || app.hover_controls || v.get_panel() || !v.get_playing();
    if v.get_controls() != show {
        v.set_controls(show);
    }
}
