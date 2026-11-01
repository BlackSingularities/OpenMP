//! Wideo zszyte z resztą programu: mpv rysuje klatki do tekstury OpenGL w rozmiarze powierzchni,
//! na której obraz jest akurat pokazywany – widok filmu (okno albo pełny ekran), miejsce okładki
//! w oknie muzyki albo miniodtwarzacz. Wyjście z widoku filmu nie przerywa odtwarzania.
//! Z tej samej tekstury powstaje poświata (obraz zmniejszony do kilkudziesięciu pikseli) i barwa
//! przewodnia, a drugi, niewidoczny odtwarzacz daje podgląd klatek nad paskiem przewijania.

use std::cell::RefCell;
use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use glow::HasContext;
use openmp_core::meta;
use openmp_core::mpv::{Mpv, Render};
use openmp_core::settings::Settings;
use slint::{ComponentHandle, GraphicsAPI, ModelRc, RenderingState, VecModel};

use crate::app::*;
use crate::*;

/// Tekstura z buforem ramki, do której rysuje mpv albo trafia zmniejszony obraz.
struct Target {
    tex: glow::Texture,
    fbo: glow::Framebuffer,
    size: (u32, u32),
}

struct Gl {
    gl: glow::Context,
    render: Render,
    main: Option<Target>,
    /// Kolejne zmniejszenia obrazu; ostatnie – poświata.
    chain: Vec<Target>,
    blit: bool,
    preview: Option<Render>,
    prev_tgt: Option<Target>,
    last_sample: Instant,
}

thread_local! {
    static GL: RefCell<Option<Gl>> = const { RefCell::new(None) };
    /// Piksele poświaty do barwy przewodniej (RGBA, szerokość, wysokość).
    static SAMPLE: RefCell<Option<(Vec<u8>, u32, u32)>> = const { RefCell::new(None) };
    /// Podgląd ma już klatkę z bieżącego pliku.
    static PREVIEW_FRAME: RefCell<bool> = const { RefCell::new(false) };
}

const HWDEC: [&str; 6] = ["no", "auto-safe", "auto-copy", "d3d11va-copy", "nvdec-copy", "dxva2-copy"];
const TONE: [&str; 6] = ["auto", "bt.2390", "hable", "mobius", "reinhard", "clip"];
const YTDL: [&str; 4] = ["bestvideo+bestaudio/best", "bestvideo[height<=?1080]+bestaudio/best", "bestvideo[height<=?720]+bestaudio/best", "bestvideo[height<=?480]+bestaudio/best"];
const SUB_AUTO: [&str; 4] = ["no", "exact", "fuzzy", "all"];
const SUB_COLOR: [&str; 4] = ["#FFFFFF", "#FFE14D", "#86E1FF", "#A6FFA0"];

/// Odtwarzacz wideo z ustawieniami programu.
pub fn create(s: &Settings) -> Result<Mpv, String> {
    let shots = s.screenshot_dir.clone();
    let _ = std::fs::create_dir_all(&shots);
    let opts: Vec<(&str, &str)> = vec![
        ("vo", "libmpv"),
        ("hwdec", HWDEC[s.hwdec_mode.clamp(0, 5) as usize]),
        ("idle", "yes"),
        ("keep-open", "no"),
        ("config", "no"),
        ("load-scripts", "no"),
        ("terminal", "no"),
        ("input-default-bindings", "no"),
        ("input-vo-keyboard", "no"),
        ("osc", "no"),
        ("osd-level", "0"),
        ("audio-display", "no"),
        ("cursor-autohide", "no"),
        ("screenshot-directory", &shots),
        ("screenshot-format", "png"),
        ("screenshot-template", "OpenMP-%F-%P"),
        ("cache", "yes"),
        ("force-seekable", "yes"),
        ("network-timeout", "20"),
        ("user-agent", "OpenMP"),
        ("title", "OpenMP"),
    ];
    let m = Mpv::new(&opts)?;
    apply_all(&m, s);
    ytdlp(&m);
    Ok(m)
}

/// yt-dlp dla linków z serwisów wideo: z systemu (PATH), a gdy go nie ma – własna kopia
/// w pamięci podręcznej programu, pobierana z oficjalnych wydań i odświeżana co tydzień.
fn ytdlp(m: &Mpv) {
    let in_path = std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join("yt-dlp.exe").exists()));
    if in_path {
        return;
    }
    let exe = openmp_core::paths::cache().join("yt-dlp.exe");
    if exe.exists() {
        m.set("script-opts", &format!("ytdl_hook-ytdl_path={}", exe.display()));
    }
    let stale = std::fs::metadata(&exe).and_then(|x| x.modified()).ok().and_then(|t| t.elapsed().ok()).map(|e| e > Duration::from_secs(7 * 86_400)).unwrap_or(true);
    if !stale {
        return;
    }
    let m = m.clone();
    std::thread::spawn(move || {
        use std::io::Read;
        let url = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";
        let Ok(resp) = ureq::get(url).set("User-Agent", "OpenMP").call() else { return };
        let mut buf = Vec::new();
        if resp.into_reader().take(64 << 20).read_to_end(&mut buf).is_err() || buf.len() < 1 << 20 || !buf.starts_with(b"MZ") {
            return;
        }
        openmp_core::paths::write_atomic(&exe, &buf);
        m.set("script-opts", &format!("ytdl_hook-ytdl_path={}", exe.display()));
    });
}

/// Drugi, niewidoczny odtwarzacz – klatki do podglądu nad paskiem przewijania.
pub fn create_preview() -> Option<Mpv> {
    let opts: Vec<(&str, &str)> = vec![
        ("vo", "libmpv"),
        ("hwdec", "auto-safe"),
        ("idle", "yes"),
        ("keep-open", "always"),
        ("pause", "yes"),
        ("config", "no"),
        ("load-scripts", "no"),
        ("terminal", "no"),
        ("input-default-bindings", "no"),
        ("osc", "no"),
        ("osd-level", "0"),
        ("ao", "null"),
        ("aid", "no"),
        ("sid", "no"),
        ("ytdl", "no"),
        ("cache", "no"),
        ("hr-seek", "no"),
        ("vd-lavc-skiploopfilter", "all"),
        ("vd-lavc-fast", "yes"),
        ("scale", "bilinear"),
        ("dscale", "bilinear"),
        ("cscale", "bilinear"),
    ];
    Mpv::new(&opts).ok()
}

/// Wszystkie ustawienia obrazu, dźwięku, sieci i napisów (przy starcie i po zmianie w ustawieniach).
pub fn apply_all(m: &Mpv, s: &Settings) {
    apply_quality(m, s);
    apply_subs(m, s);
    m.set("hwdec", HWDEC[s.hwdec_mode.clamp(0, 5) as usize]);
    m.set("deinterlace", if s.deinterlace { "yes" } else { "no" });
    m.set("tone-mapping", TONE[s.tone_mapping.clamp(0, 5) as usize]);
    m.set("volume-max", &format!("{:.0}", s.volume_boost.clamp(100.0, 200.0)));
    m.set("audio-channels", if s.downmix { "stereo" } else { "auto-safe" });
    m.set("slang", &s.sub_lang);
    m.set("alang", &s.audio_lang);
    m.set("ytdl", if s.ytdl { "yes" } else { "no" });
    m.set("ytdl-format", YTDL[s.ytdl_format.clamp(0, 3) as usize]);
    let mb = s.cache_mb.clamp(32, 2048);
    m.set("demuxer-max-bytes", &format!("{mb}MiB"));
    m.set("demuxer-max-back-bytes", &format!("{}MiB", (mb / 4).max(16)));
}

/// Jakość skalowania, usuwanie pasm i płynny ruch.
pub fn apply_quality(m: &Mpv, s: &Settings) {
    let (scale, cscale, dscale, extra) = match s.video_quality {
        0 => ("bilinear", "bilinear", "bilinear", false),
        2 => ("ewa_lanczos", "spline36", "mitchell", true),
        3 => ("ewa_lanczossharp", "ewa_lanczos", "mitchell", true),
        _ => ("spline36", "spline36", "mitchell", true),
    };
    m.set("scale", scale);
    m.set("cscale", cscale);
    m.set("dscale", dscale);
    let yn = |b: bool| if b { "yes" } else { "no" };
    m.set("correct-downscaling", yn(extra));
    m.set("linear-downscaling", yn(extra));
    m.set("sigmoid-upscaling", yn(extra));
    m.set("dither-depth", if s.video_quality == 0 { "no" } else { "auto" });
    m.set("hdr-compute-peak", if s.video_quality >= 2 { "yes" } else { "auto" });
    m.set("deband", yn(s.deband));
    if s.interpolation {
        m.set("video-sync", "display-resample");
        m.set("interpolation", "yes");
        m.set("tscale", "oversample");
    } else {
        m.set("interpolation", "no");
        m.set("video-sync", "audio");
    }
}

/// Wygląd i wczytywanie napisów.
pub fn apply_subs(m: &Mpv, s: &Settings) {
    m.set("sub-auto", SUB_AUTO[s.sub_auto.clamp(0, 3) as usize]);
    m.set("sub-font", &s.sub_font);
    m.set("sub-font-size", &s.sub_size.to_string());
    m.set("sub-color", SUB_COLOR[s.sub_color.clamp(0, 3) as usize]);
    m.set("sub-bold", if s.sub_bold { "yes" } else { "no" });
    m.set("sub-pos", &s.sub_pos.clamp(0, 150).to_string());
    m.set("sub-ass-override", if s.sub_override { "force" } else { "yes" });
    match s.sub_style {
        1 => {
            m.set("sub-border-style", "outline-and-shadow");
            m.set("sub-border-size", "0.8");
            m.set("sub-shadow-offset", "2.5");
            m.set("sub-shadow-color", "#C0000000");
            m.set("sub-back-color", "#00000000");
        }
        2 => {
            m.set("sub-border-style", "opaque-box");
            m.set("sub-border-size", "3");
            m.set("sub-outline-color", "#A0000000");
            m.set("sub-border-color", "#A0000000");
            m.set("sub-back-color", "#A0000000");
            m.set("sub-shadow-offset", "0");
        }
        _ => {
            m.set("sub-border-style", "outline-and-shadow");
            m.set("sub-border-size", "2.6");
            m.set("sub-shadow-offset", "0.8");
            m.set("sub-outline-color", "#FF000000");
            m.set("sub-border-color", "#FF000000");
            m.set("sub-shadow-color", "#80000000");
            m.set("sub-back-color", "#00000000");
        }
    }
}

/// Podpina rysowanie mpv (i podglądu) do okna.
pub fn attach(ui: &AppWindow, mpv: Mpv, preview: Option<Mpv>) {
    let weak = ui.as_weak();
    let res = ui.window().set_rendering_notifier(move |state, api| match state {
        RenderingState::RenderingSetup => {
            if let GraphicsAPI::NativeOpenGL { get_proc_address } = api {
                let gl = unsafe { glow::Context::from_loader_function_cstr(|s| get_proc_address(s)) };
                let redraw = || -> Box<dyn Fn() + Send + Sync> {
                    let w = weak.clone();
                    Box::new(move || {
                        let w = w.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = w.upgrade() {
                                ui.window().request_redraw();
                            }
                        });
                    })
                };
                let blit = gl.version().major >= 3;
                let prev = preview.as_ref().and_then(|p| Render::new(p, get_proc_address, redraw()).ok());
                match Render::new(&mpv, get_proc_address, redraw()) {
                    Ok(render) => GL.with(|g| {
                        *g.borrow_mut() = Some(Gl { gl, render, main: None, chain: Vec::new(), blit, preview: prev, prev_tgt: None, last_sample: Instant::now() })
                    }),
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
                if let Some(mut g) = g.borrow_mut().take() {
                    unsafe {
                        if let Some(t) = g.main.take() {
                            free(&g.gl, t);
                        }
                        for t in g.chain.drain(..) {
                            free(&g.gl, t);
                        }
                        if let Some(t) = g.prev_tgt.take() {
                            free(&g.gl, t);
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

unsafe fn target(gl: &glow::Context, w: u32, h: u32) -> Option<Target> {
    let tex = gl.create_texture().ok()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(tex));
    gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, w as i32, h as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(None));
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
    let fbo = gl.create_framebuffer().ok()?;
    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
    gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(tex), 0);
    gl.clear_color(0.0, 0.0, 0.0, 1.0);
    gl.clear(glow::COLOR_BUFFER_BIT);
    Some(Target { tex, fbo, size: (w, h) })
}

unsafe fn free(gl: &glow::Context, t: Target) {
    gl.delete_framebuffer(t.fbo);
    gl.delete_texture(t.tex);
}

fn image_of(t: &Target) -> slint::Image {
    unsafe { slint::BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(t.tex.0, t.size.into()).origin(slint::BorrowedOpenGLTextureOrigin::TopLeft).build() }
}

/// Rozmiary kolejnych zmniejszeń obrazu – co najwyżej 4× na krok, żeby poświata nie migotała.
fn chain_sizes(w: u32, h: u32) -> Vec<(u32, u32)> {
    // Piramida: w dół co 2× do ~24 px (każdy krok uśrednia 2×2), potem w górę co 2× do ~200 px –
    // dwuliniowe powiększanie krok po kroku daje miękkie rozmycie bez schodków i migotania.
    let mut v = Vec::new();
    let (mut cw, mut ch) = (w, h);
    while cw / 2 >= 24 {
        cw /= 2;
        ch = (ch / 2).max(1);
        v.push((cw, ch));
    }
    let mut up = v.len();
    while up >= 2 && v[up - 2].0 <= 200 {
        up -= 1;
        v.push(v[up - 1]);
    }
    if v.is_empty() {
        v.push((w.max(1), h.max(1)));
    }
    v
}

fn draw(ui: &AppWindow) {
    let v = ui.global::<Video>();
    let pr = ui.global::<Prefs>();
    let active = v.get_active();
    let want_ambient = pr.get_ambient() > 0;
    let want_tint = pr.get_video_tint();
    let tint_every = Duration::from_millis([500, 1200, 3000, 8000][pr.get_tint_rate().clamp(0, 3) as usize]);
    let sf = ui.window().scale_factor();
    let win = ui.window().size();
    let (sw, sh) = (v.get_surface_w() * sf, v.get_surface_h() * sf);
    let (w, h) = if sw >= 2.0 && sh >= 2.0 { (sw.round() as u32, sh.round() as u32) } else { (win.width.max(1), win.height.max(1)) };
    let ratio = v.get_ratio().max(0.2);
    GL.with(|g| {
        let mut g = g.borrow_mut();
        let Some(g) = g.as_mut() else { return };
        unsafe {
            let gl = &g.gl;
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
            gl.disable(glow::SCISSOR_TEST);

            // Obraz filmu w rozmiarze powierzchni.
            let mut drew = false;
            if active {
                let mut force = false;
                if g.main.as_ref().map(|t| t.size) != Some((w, h)) {
                    if let Some(t) = g.main.take() {
                        free(gl, t);
                    }
                    if let Some(t) = target(gl, w, h) {
                        v.set_frame(image_of(&t));
                        v.set_has_frame(true);
                        g.main = Some(t);
                        force = true;
                    }
                }
                if g.render.needs_frame() || force {
                    if let Some(t) = &g.main {
                        g.render.draw(t.fbo.0.get() as i32, w as i32, h as i32);
                        drew = true;
                    }
                }
            } else {
                // Bez widocznego obrazu też odbieramy klatki, by mpv nie czekał na rysowanie.
                let _ = g.render.needs_frame();
            }

            // Poświata: obraz zmniejszany krokami do 40 pikseli szerokości.
            if drew && g.blit && (want_ambient || want_tint) {
                let sizes = chain_sizes(w, h);
                if g.chain.iter().map(|t| t.size).collect::<Vec<_>>() != sizes {
                    for t in g.chain.drain(..) {
                        free(gl, t);
                    }
                    for (cw, ch) in &sizes {
                        if let Some(t) = target(gl, *cw, *ch) {
                            g.chain.push(t);
                        }
                    }
                    if let Some(last) = g.chain.last() {
                        v.set_ambient(image_of(last));
                    }
                }
                if let Some(main) = &g.main {
                    let mut src = (main.fbo, main.size);
                    for t in &g.chain {
                        gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(src.0));
                        gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(t.fbo));
                        gl.blit_framebuffer(0, 0, src.1 .0 as i32, src.1 .1 as i32, 0, 0, t.size.0 as i32, t.size.1 as i32, glow::COLOR_BUFFER_BIT, glow::LINEAR);
                        src = (t.fbo, t.size);
                    }
                    if !v.get_has_ambient() && !g.chain.is_empty() {
                        v.set_has_ambient(true);
                    }
                }
                // Barwa przewodnia z obrazu – rzadko, odczyt zatrzymuje na chwilę kartę graficzną.
                if want_tint && g.last_sample.elapsed() > tint_every {
                    // Najmniejszy stopień piramidy – mało pikseli do odczytu.
                    if let Some(last) = g.chain.iter().min_by_key(|t| t.size.0) {
                        let (lw, lh) = last.size;
                        let mut buf = vec![0u8; (lw * lh * 4) as usize];
                        gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(last.fbo));
                        gl.read_pixels(0, 0, lw as i32, lh as i32, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(Some(&mut buf[..])));
                        SAMPLE.with(|s| *s.borrow_mut() = Some((buf, lw, lh)));
                        g.last_sample = Instant::now();
                    }
                }
            }

            // Podgląd klatki nad paskiem przewijania.
            if let Some(p) = &g.preview {
                if p.needs_frame() {
                    let pw = 256u32;
                    let ph = ((pw as f32 / ratio).round() as u32).clamp(72, 256);
                    if g.prev_tgt.as_ref().map(|t| t.size) != Some((pw, ph)) {
                        if let Some(t) = g.prev_tgt.take() {
                            free(gl, t);
                        }
                        if let Some(t) = target(gl, pw, ph) {
                            v.set_preview(image_of(&t));
                            g.prev_tgt = Some(t);
                        }
                    }
                    if let Some(t) = &g.prev_tgt {
                        p.draw(t.fbo.0.get() as i32, pw as i32, ph as i32);
                        PREVIEW_FRAME.with(|f| *f.borrow_mut() = true);
                    }
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

/// Czy gra film z obrazem (nie tylko dźwięk).
fn picture_on(app: &App) -> bool {
    let bg = app.ui().map(|u| u.global::<Video>().get_bg_audio()).unwrap_or(false);
    app.cur.as_ref().is_some_and(|c| c.video && c.item != 0) && !bg
}

/// Obraz pokazywany (widok filmu, miejsce okładki, miniodtwarzacz) albo nie.
pub fn refresh_active(app: &mut App) {
    let on = picture_on(app);
    if let Some(ui) = app.ui() {
        let v = ui.global::<Video>();
        let on = on && !v.get_audio_only();
        if v.get_active() != on {
            v.set_active(on);
            if !on {
                v.set_has_ambient(false);
            }
        }
        ui.window().request_redraw();
    }
}

/// Wejście do widoku filmu (tryb 1) – pełny ekran z samym obrazem.
pub fn enter_video(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    let u = ui.global::<Ui>();
    let main_fs = u.get_fullscreen();
    let from = u.get_mode();
    if from != 1 {
        if from == 0 && !main_fs && !ui.window().is_maximized() {
            app.music_geom = Some(crate::win::geometry(&ui));
        }
        u.set_mode(1);
        crate::win::set_ontop(app, ui.global::<Video>().get_ontop());
    }
    let v = ui.global::<Video>();
    if !v.get_fullscreen() {
        if !main_fs {
            crate::win::fullscreen_frame(&ui, true);
            ui.window().set_fullscreen(true);
        }
        v.set_fullscreen(true);
    }
    v.set_controls(true);
    v.set_error(ss(""));
    app.last_mouse = Instant::now();
    ui.window().request_redraw();
}

/// Wyjście z pełnego ekranu filmu do okna – bez zatrzymywania odtwarzania; film gra dalej
/// w miejscu okładki, obok opisu. Okno na pełnym ekranie zostaje na pełnym ekranie.
pub fn leave_video(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    let u = ui.global::<Ui>();
    if u.get_mode() != 1 {
        return;
    }
    let main_fs = u.get_fullscreen();
    let v = ui.global::<Video>();
    if v.get_fullscreen() {
        if !main_fs {
            crate::win::fullscreen_frame(&ui, false);
            ui.window().set_fullscreen(false);
        }
        v.set_fullscreen(false);
    }
    v.set_panel(false);
    u.set_mode(0);
    crate::win::set_ontop(app, app.s.always_on_top);
    if !main_fs {
        if let Some(g) = app.music_geom.take() {
            crate::win::set_geometry(&ui, g);
        }
    }
}

/// Przycisk „Biblioteka” / Esc: wyjście z widoku filmu według ustawień.
pub fn leave_view(app: &mut App) {
    match app.s.video_leave {
        1 => hide_picture(app),
        2 => crate::player::set_paused(app, true),
        _ => {}
    }
    leave_video(app);
    if app.s.video_leave == 1 && app.cur.as_ref().is_some_and(|c| c.video) {
        app.toast(tr!("The movie keeps playing as audio – click the cover to return to the picture"));
    }
}

/// Powrót do obrazu filmu (z miejsca okładki, miniodtwarzacza albo z samego dźwięku).
pub fn open_view(app: &mut App) {
    if !app.cur.as_ref().is_some_and(|c| c.video) {
        return;
    }
    show_picture(app);
    enter_video(app);
}

/// Film gra dalej samym dźwiękiem – dekodowanie obrazu wyłączone.
pub fn hide_picture(app: &mut App) {
    let Some(m) = app.mpv.clone() else { return };
    if !app.cur.as_ref().is_some_and(|c| c.video) {
        return;
    }
    m.set("vid", "no");
    if let Some(ui) = app.ui() {
        ui.global::<Video>().set_bg_audio(true);
    }
    app.engine.set_loopback(true);
    refresh_active(app);
}

pub fn show_picture(app: &mut App) {
    let Some(m) = app.mpv.clone() else { return };
    let bg = app.ui().map(|u| u.global::<Video>().get_bg_audio()).unwrap_or(false);
    if bg {
        m.set("vid", "auto");
        if let Some(ui) = app.ui() {
            ui.global::<Video>().set_bg_audio(false);
        }
        app.engine.set_loopback(false);
    }
    refresh_active(app);
}

/// Utwór spoza mpv – znikają ścieżki, rozdziały i podgląd poprzedniego filmu.
pub fn clear(app: &mut App) {
    app.chapters.clear();
    app.preview_file.clear();
    app.preview_want = None;
    if let Some(ui) = app.ui() {
        let v = ui.global::<Video>();
        v.set_chapters(ModelRc::new(VecModel::from(Vec::<f32>::new())));
        v.set_chapter_list(ModelRc::new(VecModel::from(Vec::<Opt>::new())));
        v.set_audio_tracks(ModelRc::new(VecModel::from(Vec::<Opt>::new())));
        v.set_sub_tracks(ModelRc::new(VecModel::from(Vec::<Opt>::new())));
        v.set_has_preview(false);
        v.set_bg_audio(false);
        v.set_buffered(0.0);
    }
    refresh_active(app);
}

/// Pełny ekran filmu (sam obraz) – włączenie z okna, wyłączenie z powrotem do okna.
pub fn toggle_fullscreen(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    if ui.global::<Ui>().get_mode() == 1 {
        leave_video(app);
    } else {
        open_view(app);
    }
}

/// Plik wczytany: ścieżki, napisy, rozdziały, opis, odświeżenie podglądu.
pub fn on_loaded(app: &mut App) {
    let Some(m) = app.mpv.clone() else { return };
    let Some(ui) = app.ui() else { return };
    let v = ui.global::<Video>();
    v.set_loading(false);
    v.set_is_url(app.cur.as_ref().is_some_and(|c| meta::is_url(&c.info.path)));
    // Ustawienia zapamiętane dla tego filmu: ścieżki dźwięku i napisów, przesunięcia, proporcje.
    let saved = app.cur.as_ref().filter(|c| c.video).map(|c| app.stats.get(&c.info.path));
    if let Some(st) = &saved {
        if !st.aid.is_empty() {
            m.set("aid", &st.aid);
        }
        if !st.sid.is_empty() {
            m.set("sid", &st.sid);
        }
    }
    let tracks = m.tracks();
    let has_video = tracks.iter().any(|t| t.kind == "video" && !t.title.contains("cover"));
    let is_video = app.cur.as_ref().is_some_and(|c| c.video);
    let bg = v.get_bg_audio();
    v.set_audio_only(is_video && !has_video && !bg);
    publish_tracks(app, &tracks);
    if is_video {
        crate::display::sub_panel_setup(app, &tracks);
    }
    let dur = m.get_f64("duration").unwrap_or(0.0);
    app.chapters = m.chapters();
    let chapters = app.chapters.clone();
    v.set_chapters(ModelRc::new(VecModel::from(if dur > 0.0 { chapters.iter().map(|c| (c.0 / dur) as f32).collect::<Vec<_>>() } else { Vec::new() })));
    v.set_chapter_list(ModelRc::new(VecModel::from(
        chapters
            .iter()
            .enumerate()
            .map(|(i, (t, n))| Opt { id: ss(i.to_string()), label: ss(if n.is_empty() { tr!("Chapter {}", i + 1) } else { n.clone() }), sub: ss(meta::fmt_time(*t)), selected: false })
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
    let (sd, ad, aspect) = saved.as_ref().map(|s| (s.sub_delay, s.audio_delay, s.aspect)).unwrap_or((0.0, 0.0, 0));
    v.set_sub_delay(sd as f32);
    v.set_audio_delay(ad as f32);
    m.set_f64("sub-delay", sd);
    m.set_f64("audio-delay", ad);
    if v.get_aspect() != aspect {
        set_aspect(app, aspect);
    }
    crate::player::apply_volume(app);
    if let Some(c) = &mut app.cur {
        if c.info.duration <= 0.0 && dur > 0.0 {
            c.info.duration = dur;
        }
    }
    // Wznowiony film – komunikat, skąd gra.
    let pos = m.get_f64("time-pos").unwrap_or(0.0);
    if is_video && pos > 10.0 && app.is_video_mode() {
        app.osd(tr!("Resumed from {}  ·  Home – from the start", meta::fmt_time(pos)));
    }
    // Podgląd klatek – ten sam plik w drugim, niewidocznym odtwarzaczu.
    app.preview_file.clear();
    app.preview_want = None;
    PREVIEW_FRAME.with(|f| *f.borrow_mut() = false);
    v.set_has_preview(false);
    refresh_active(app);
}

fn publish_tracks(app: &App, tracks: &[openmp_core::mpv::TrackEntry]) {
    let Some(ui) = app.ui() else { return };
    let v = ui.global::<Video>();
    let mut audio: Vec<Opt> = Vec::new();
    let mut subs: Vec<Opt> = vec![Opt { id: ss("no"), label: ss(tr!("Off")), sub: ss(""), selected: !tracks.iter().any(|t| t.kind == "sub" && t.selected) }];
    for t in tracks {
        let mut label = if t.title.is_empty() { tr!("Track {}", t.id) } else { t.title.clone() };
        if !t.lang.is_empty() {
            label = format!("{label} [{}]", t.lang);
        }
        let mut sub = t.codec.to_uppercase();
        if t.external {
            sub.push_str(&format!(" · {}", tr!("external file")));
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
}

/// Opis obrazu: rozdzielczość, kodek, klatki, HDR, dekodowanie – też jako plakietki przy tytule.
fn info_line(app: &mut App) {
    let Some(m) = &app.mpv else { return };
    let is_video = app.cur.as_ref().is_some_and(|c| c.video);
    let mut parts = Vec::new();
    let mut badges = Vec::new();
    if let (Some(w), Some(h)) = (m.get_i64("width"), m.get_i64("height")) {
        if w > 0 {
            parts.push(format!("{w}×{h}"));
            badges.push(match h {
                h if h >= 2000 => "4K".to_string(),
                h if h >= 1400 => "1440p".to_string(),
                h if h >= 1000 => "1080p".to_string(),
                h if h >= 700 => "720p".to_string(),
                h => format!("{h}p"),
            });
        }
    }
    if let Some(c) = m.get_str("video-codec").filter(|c| !c.is_empty()) {
        let c = c.split(' ').next().unwrap_or("").to_uppercase();
        badges.push(c.clone());
        parts.push(c);
    }
    let gamma = m.get_str("video-params/gamma").unwrap_or_default();
    if gamma == "pq" || gamma == "hlg" {
        let hdr = if gamma == "pq" { "HDR10" } else { "HLG" };
        parts.push(hdr.into());
        badges.push(hdr.into());
    }
    if let Some(f) = m.get_f64("container-fps").filter(|f| *f > 0.0) {
        let s = tr!("{} fps", num(f, 3));
        badges.push(s.clone());
        parts.push(s);
    }
    if let Some(hw) = m.get_str("hwdec-current").filter(|h| !h.is_empty() && h != "no") {
        parts.push(tr!("hardware ({})", hw));
    }
    if let Some(ui) = app.ui() {
        ui.global::<Video>().set_info(ss(parts.join("  ·  ")));
        if is_video && !badges.is_empty() {
            crate::display::set_badges(&ui.global::<Player>(), &badges);
        }
    }
}

/// Nazwy proporcji obrazu – w kolejności ustawienia `aspect`.
pub fn aspect_names() -> [&'static str; 6] {
    [tr!("automatic"), "16:9", "4:3", "21:9", tr!("stretched"), tr!("fill screen")]
}

/// Liczba z przecinkiem dziesiętnym, bez zbędnych zer na końcu.
fn num(v: f64, prec: usize) -> String {
    let s = format!("{v:.prec$}");
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
    s.replace('.', ",")
}

/// Szczegóły odtwarzania (karta „Info”), odświeżane co pół sekundy, gdy są widoczne.
fn stats(app: &mut App) {
    let Some(m) = app.mpv.clone() else { return };
    let mut v: Vec<(String, String)> = Vec::new();
    let mut add = |l: &str, s: String| {
        if !s.trim().is_empty() {
            v.push((l.into(), s));
        }
    };
    if let Some(c) = &app.cur {
        add(tr!("File"), std::path::Path::new(&c.info.path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| c.info.path.clone()));
    }
    add(tr!("Container"), m.get_str("file-format").unwrap_or_default().to_uppercase());
    if let (Some(w), Some(h)) = (m.get_i64("width"), m.get_i64("height")) {
        let mut s = format!("{w}×{h}");
        if let (Some(dw), Some(dh)) = (m.get_i64("dwidth"), m.get_i64("dheight")) {
            if (dw, dh) != (w, h) {
                s.push_str(&format!(" → {dw}×{dh}"));
            }
        }
        add(tr!("Resolution"), s);
    }
    add(tr!("Video codec"), m.get_str("video-format").or_else(|| m.get_str("video-codec")).unwrap_or_default());
    let fps = m.get_f64("container-fps").unwrap_or(0.0);
    let real = m.get_f64("estimated-vf-fps").unwrap_or(0.0);
    if fps > 0.0 {
        add(tr!("Frames"), tr!("{} fps (displayed {})", num(fps, 3), num(real, 2)));
    }
    add(tr!("Decoding"), match m.get_str("hwdec-current").filter(|h| !h.is_empty() && h != "no") {
        Some(h) => tr!("hardware – {}", h),
        None => tr!("software (CPU)").into(),
    });
    let prim = m.get_str("video-params/primaries").unwrap_or_default();
    let gamma = m.get_str("video-params/gamma").unwrap_or_default();
    if !prim.is_empty() {
        add(tr!("Colors"), format!("{prim} · {gamma}"));
    }
    let mbps = |b: f64| format!("{} Mb/s", num(b / 1e6, 2));
    if let Some(b) = m.get_f64("video-bitrate").filter(|b| *b > 0.0) {
        add(tr!("Video bitrate"), mbps(b));
    }
    let ac = m.get_str("audio-codec-name").unwrap_or_default().to_uppercase();
    let rate = m.get_i64("audio-params/samplerate").unwrap_or(0);
    let ch = m.get_i64("audio-params/channel-count").unwrap_or(0);
    if !ac.is_empty() {
        add(tr!("Audio"), format!("{ac} · {} kHz · {}", num(rate as f64 / 1000.0, 1), tr!("{} ch.", ch)));
    }
    if let Some(b) = m.get_f64("audio-bitrate").filter(|b| *b > 0.0) {
        add(tr!("Audio bitrate"), format!("{:.0} kb/s", b / 1000.0));
    }
    let drop = m.get_i64("frame-drop-count").unwrap_or(0) + m.get_i64("decoder-frame-drop-count").unwrap_or(0);
    add(tr!("Dropped frames"), drop.to_string());
    if let Some(c) = m.get_f64("demuxer-cache-duration") {
        add(tr!("Buffer"), format!("{} s", num(c, 1)));
    }
    if let Some(a) = m.get_f64("avsync") {
        add(tr!("A/V sync"), format!("{:+.0} ms", a * 1000.0));
    }
    let rows: Vec<Opt> = v.into_iter().map(|(l, s)| Opt { id: ss(""), label: ss(l), sub: ss(s), selected: false }).collect();
    if let Some(ui) = app.ui() {
        ui.global::<Video>().set_stats(ModelRc::new(VecModel::from(rows)));
    }
}

/// Zmiana rozmiaru obrazu: proporcje dla interfejsu, okno dopasowuje się do filmu (najwyżej 80% ekranu).
pub fn on_reconfig(app: &mut App) {
    info_line(app);
    let Some(m) = app.mpv.clone() else { return };
    let (Some(w), Some(h)) = (m.get_i64("dwidth"), m.get_i64("dheight")) else { return };
    if w <= 0 || h <= 0 || (w, h) == app.video_size {
        return;
    }
    app.video_size = (w, h);
    let Some(ui) = app.ui() else { return };
    ui.global::<Video>().set_ratio(w as f32 / h as f32);
    let mode = ui.global::<Ui>().get_mode();
    if mode == 2 {
        crate::keys::fit_mini(app);
    }
}

/// Filtry dźwięku mpv – te same co dla muzyki: korektor (firequalizer), korektor parametryczny,
/// przestrzeń (balans, szerokość, mono, zamiana kanałów, przesłuch), ogranicznik i tryb nocny.
pub fn apply_audio_filters(app: &App) {
    let Some(m) = &app.mpv else { return };
    let d = &app.s.dsp;
    let mut f: Vec<String> = Vec::new();
    // Przestrzeń: najpierw układ kanałów, potem szerokość i balans.
    if d.swap {
        f.push("lavfi=[pan=stereo|c0=c1|c1=c0]".into());
    }
    if d.mono {
        f.push("lavfi=[pan=stereo|c0=0.5*c0+0.5*c1|c1=0.5*c0+0.5*c1]".into());
    } else if (d.width - 1.0).abs() > 0.01 {
        f.push(format!("lavfi=[extrastereo=m={:.2}:c=0]", d.width.clamp(0.0, 2.0)));
    }
    if d.balance.abs() > 0.01 {
        f.push(format!("lavfi=[stereotools=balance_out={:.2}]", d.balance.clamp(-1.0, 1.0)));
    }
    if d.crossfeed {
        f.push("lavfi=[crossfeed]".into());
    }
    if d.eq_on {
        let entries: Vec<String> = openmp_core::audio::dsp::BANDS.iter().zip(d.gains.iter()).map(|(f, g)| format!("entry({f},{g:.1})")).collect();
        f.push(format!("lavfi=[firequalizer=gain_entry='{}']", entries.join(";")));
        if d.preamp.abs() > 0.05 {
            f.push(format!("lavfi=[volume={:.1}dB]", d.preamp));
        }
    }
    if d.peq_on && !d.peq.is_empty() {
        if d.peq_pre.abs() > 0.05 {
            f.push(format!("lavfi=[volume={:.1}dB]", d.peq_pre));
        }
        for p in &d.peq {
            let name = match p.kind {
                1 => "lowshelf",
                2 => "highshelf",
                _ => "equalizer",
            };
            f.push(format!("lavfi=[{name}=f={:.1}:width_type=q:w={:.3}:g={:.2}]", p.f, p.q, p.db));
        }
    }
    if app.s.night_mode {
        f.push("lavfi=[dynaudnorm=f=200:g=15:p=0.7]".into());
    }
    // Ogranicznik na końcu – chroni przed przesterowaniem po podbiciu.
    if d.limiter {
        f.push("lavfi=[alimiter=limit=0.97:level=disabled]".into());
    }
    m.set("af", &f.join(","));
}

pub fn command(app: &mut App, id: &str) {
    match id {
        "v-pause" => crate::player::play_pause(app),
        "v-full" => toggle_fullscreen(app),
        "v-ontop" => toggle_ontop(app),
        "v-audio" => open_panel(app, 0),
        "v-subs" => open_panel(app, 1),
        "v-image" => open_panel(app, 2),
        "v-chapters" => open_panel(app, 3),
        "v-info" => open_panel(app, 4),
        "v-shot" => screenshot(app, true),
        "v-shot-clean" => screenshot(app, false),
        "v-ab" => ab_loop(app),
        "v-back" => leave_view(app),
        "v-audio-only" => {
            hide_picture(app);
            leave_video(app);
        }
        "v-mini" => crate::keys::set_mode(app, 2),
        "v-rotate" => rotate(app),
        "v-aspect-next" => cycle_aspect(app),
        "v-reset" => reset_view(app),
        "v-zoom-in" => zoom_by(app, 0.25),
        "v-zoom-out" => zoom_by(app, -0.25),
        "v-deband" => toggle_deband(app),
        "v-ambient" => {
            app.s.ambient = if app.s.ambient > 0 { 0 } else { 1 };
            if let Some(ui) = app.ui() {
                ui.global::<Prefs>().set_ambient(app.s.ambient);
            }
            app.osd(if app.s.ambient > 0 { tr!("Ambient glow on") } else { tr!("Ambient glow off") });
            app.dirty = true;
        }
        "open-files" => crate::dialogs::open_files(app),
        _ => {
            if let Some(i) = id.strip_prefix("v-aspect-").and_then(|s| s.parse::<i32>().ok()) {
                set_aspect(app, i);
            }
        }
    }
}

pub fn open_panel(app: &mut App, page: i32) {
    let Some(ui) = app.ui() else { return };
    if ui.global::<Ui>().get_mode() != 1 {
        open_view(app);
    }
    let v = ui.global::<Video>();
    v.set_panel_page(page);
    if !v.get_fullscreen() {
        v.set_side(true);
    } else {
        v.set_panel(true);
    }
    if page == 4 {
        stats(app);
    }
}

pub fn toggle_ontop(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    let on = !ui.global::<Video>().get_ontop();
    ui.global::<Video>().set_ontop(on);
    crate::win::set_ontop(app, on);
    app.osd(if on { tr!("Always on top") } else { tr!("Normal window") });
}

pub fn screenshot(app: &mut App, with_subs: bool) {
    if let Some(m) = &app.mpv {
        let _ = std::fs::create_dir_all(&app.s.screenshot_dir);
        m.set("screenshot-directory", &app.s.screenshot_dir);
        m.command_async(&["screenshot", if with_subs { "subtitles" } else { "video" }]);
    }
    app.osd(if with_subs { tr!("Screenshot saved") } else { tr!("Screenshot (without subtitles) saved") });
}

pub fn ab_loop(app: &mut App) {
    let (pos, dur) = crate::player::position(app);
    app.ab = match app.ab {
        (None, _) => (Some(pos), None),
        (Some(a), None) if pos > a => (Some(a), Some(pos)),
        _ => (None, None),
    };
    let msg = match app.ab {
        (Some(a), None) => tr!("A-B loop: start {}", meta::fmt_time(a)),
        (Some(a), Some(b)) => tr!("A-B loop: {} – {}", meta::fmt_time(a), meta::fmt_time(b)),
        _ => tr!("A-B loop off").into(),
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
    let Some(m) = app.mpv.clone() else { return };
    m.set(kind, id);
    publish_tracks(app, &m.tracks());
    // Wybór zapamiętany dla filmu od razu (nie dopiero przy pauzie czy zamknięciu).
    crate::player::save_resume(app);
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
            app.osd(tr!("Speed {}×", openmp_core::i18n::decimal(format!("{v:.2}").trim_end_matches('0').trim_end_matches('.'))));
        }
        "sub-delay" => {
            m.set_f64("sub-delay", v as f64);
            vg.set_sub_delay(v);
            app.osd(tr!("Subtitles: {} ms", format!("{:+}", (v * 1000.0).round())));
        }
        "audio-delay" => {
            m.set_f64("audio-delay", v as f64);
            vg.set_audio_delay(v);
            app.osd(tr!("Audio: {} ms", format!("{:+}", (v * 1000.0).round())));
        }
        "sub-scale" => {
            m.set_f64("sub-scale", v as f64);
            vg.set_sub_scale(v);
        }
        "brightness" | "contrast" | "saturation" | "gamma" | "hue" => {
            m.set_f64(what, v.round() as f64);
            match what {
                "brightness" => vg.set_brightness(v.round()),
                "contrast" => vg.set_contrast(v.round()),
                "saturation" => vg.set_saturation(v.round()),
                "hue" => vg.set_hue(v.round()),
                _ => vg.set_gamma(v.round()),
            }
        }
        "zoom" => {
            let z = v.clamp(-2.0, 3.0);
            m.set_f64("video-zoom", z as f64);
            vg.set_zoom(z);
            app.osd(tr!("Zoom {}%", (2f32.powf(z) * 100.0).round()));
        }
        _ => {}
    }
}

pub fn zoom_by(app: &mut App, d: f32) {
    let z = app.ui().map(|u| u.global::<Video>().get_zoom()).unwrap_or(0.0);
    set(app, "zoom", z + d);
}

/// Przesunięcie powiększonego obrazu.
pub fn pan(app: &mut App, dx: f64, dy: f64) {
    let Some(m) = &app.mpv else { return };
    let x = (m.get_f64("video-pan-x").unwrap_or(0.0) + dx).clamp(-3.0, 3.0);
    let y = (m.get_f64("video-pan-y").unwrap_or(0.0) + dy).clamp(-3.0, 3.0);
    m.set_f64("video-pan-x", x);
    m.set_f64("video-pan-y", y);
}

pub fn rotate(app: &mut App) {
    let Some(m) = app.mpv.clone() else { return };
    let r = (app.ui().map(|u| u.global::<Video>().get_rotate()).unwrap_or(0) + 90) % 360;
    m.set("video-rotate", &r.to_string());
    if let Some(ui) = app.ui() {
        ui.global::<Video>().set_rotate(r);
    }
    app.osd(tr!("Rotation {}°", r));
}

/// Obraz jak w pliku: bez powiększenia, przesunięcia, obrotu i zmiany proporcji.
pub fn reset_view(app: &mut App) {
    let Some(m) = app.mpv.clone() else { return };
    m.set_f64("video-zoom", 0.0);
    m.set_f64("video-pan-x", 0.0);
    m.set_f64("video-pan-y", 0.0);
    m.set("video-rotate", "0");
    if let Some(ui) = app.ui() {
        let v = ui.global::<Video>();
        v.set_zoom(0.0);
        v.set_rotate(0);
    }
    set_aspect(app, 0);
    app.osd(tr!("Picture as in file"));
}

pub fn toggle_deband(app: &mut App) {
    app.s.deband = !app.s.deband;
    if let Some(m) = &app.mpv {
        m.set("deband", if app.s.deband { "yes" } else { "no" });
    }
    if let Some(ui) = app.ui() {
        ui.global::<Prefs>().set_deband(app.s.deband);
        ui.global::<Video>().set_deband(app.s.deband);
    }
    app.osd(if app.s.deband { tr!("Debanding: on") } else { tr!("Debanding: off") });
    app.dirty = true;
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
    m.set_f64("panscan", if i == 5 { 1.0 } else { 0.0 });
    if let Some(ui) = app.ui() {
        ui.global::<Video>().set_aspect(i);
    }
}

pub fn cycle_aspect(app: &mut App) {
    let i = (app.ui().map(|u| u.global::<Video>().get_aspect()).unwrap_or(0) + 1) % 6;
    set_aspect(app, i);
    app.osd(tr!("Aspect ratio: {}", aspect_names()[i as usize]));
}

pub fn cycle_track(app: &mut App, kind: &str) {
    let Some(m) = &app.mpv else { return };
    m.command_async(&["cycle", kind]);
    let is_sub = kind == "sid";
    let label = if is_sub { tr!("Subtitles") } else { tr!("Audio track") };
    let m2 = m.clone();
    slint::Timer::single_shot(Duration::from_millis(150), move || {
        with(|app| {
            let tracks = m2.tracks();
            publish_tracks(app, &tracks);
            crate::player::save_resume(app);
            let cur = tracks.into_iter().find(|t| t.selected && t.kind == if is_sub { "sub" } else { "audio" });
            let text = match cur {
                Some(t) => format!("{label}: {}", if t.title.is_empty() { if t.lang.is_empty() { t.id.to_string() } else { t.lang } } else { t.title }),
                None => format!("{label}: {}", tr!("off")),
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

/// Polskie napisy z OpenSubtitles – w tle, potem dołączone do filmu.
pub fn fetch_sub(app: &mut App) {
    let Some(c) = app.cur.clone().filter(|c| c.video && !meta::is_url(&c.info.path)) else { return };
    if !openmp_core::opensubs::has_key() {
        app.osd(tr!("Enter an OpenSubtitles key in Settings → Library"));
        return;
    }
    app.osd(tr!("Searching for subtitles…"));
    let path = c.info.path.clone();
    std::thread::spawn(move || {
        let res = openmp_core::opensubs::fetch(std::path::Path::new(&path), "pl");
        let _ = slint::invoke_from_event_loop(move || {
            later(move |app| match res {
                Ok(p) => {
                    if let Some(m) = app.mpv.clone() {
                        let _ = m.command(&["sub-add", &p.to_string_lossy(), "select"]);
                        let tracks = m.tracks();
                        publish_tracks(app, &tracks);
                        app.sub_panel_id = 0;
                        crate::display::sub_panel_setup(app, &tracks);
                    }
                    app.osd(tr!("Subtitles downloaded"));
                }
                Err(e) => app.osd(e),
            });
        });
    });
}

pub fn load_sub(app: &mut App) {
    let dir = app.cur.as_ref().and_then(|c| std::path::Path::new(&c.info.path).parent().map(|p| p.to_path_buf()));
    crate::dialogs::ask(
        app,
        move |d| {
            let mut d = d.set_title(tr!("Load subtitles")).add_filter(tr!("Subtitles"), openmp_core::meta::SUBTITLES);
            if let Some(dir) = dir {
                d = d.set_directory(dir);
            }
            d.pick_file()
        },
        |app, p| {
            if let Some(m) = app.mpv.clone() {
                let _ = m.command(&["sub-add", &p.to_string_lossy(), "select"]);
                let tracks = m.tracks();
                publish_tracks(app, &tracks);
                app.sub_panel_id = 0;
                crate::display::sub_panel_setup(app, &tracks);
            }
        },
    );
}

pub fn frame_step(app: &mut App, back: bool) {
    if let Some(m) = &app.mpv {
        m.command_async(&[if back { "frame-back-step" } else { "frame-step" }]);
    }
}

/// Kursor nad paskiem przewijania (ułamek długości; ujemny – kursor zjechał).
pub fn hover_seek(app: &mut App, frac: f32) {
    if frac < 0.0 {
        app.preview_want = None;
        return;
    }
    let (_, dur) = crate::player::position(app);
    if dur <= 0.0 {
        return;
    }
    let t = frac as f64 * dur;
    app.preview_want = Some(t);
    let name = app.chapters.iter().rev().find(|(ct, _)| *ct <= t).map(|(_, n)| n.clone()).unwrap_or_default();
    if let Some(ui) = app.ui() {
        let v = ui.global::<Video>();
        if v.get_preview_chapter().as_str() != name {
            v.set_preview_chapter(ss(name));
        }
    }
}

fn preview_tick(app: &mut App) {
    let Some(p) = app.preview.clone() else { return };
    // Zdarzenia podglądu nie są potrzebne – kolejka nie może się jednak zapełnić.
    let _ = p.poll();
    let path = match &app.cur {
        Some(c) if c.video && c.backend == Backend::Mpv && !meta::is_url(&c.info.path) && app.s.seek_preview => c.info.path.clone(),
        _ => return,
    };
    let Some(want) = app.preview_want else { return };
    if app.preview_file != path {
        p.command_async(&["loadfile", &path, "replace"]);
        app.preview_file = path;
        app.preview_sent = -1.0;
        app.preview_at = Instant::now();
        return;
    }
    if (want - app.preview_sent).abs() > 0.4 && app.preview_at.elapsed() > Duration::from_millis(70) {
        p.command_async(&["seek", &format!("{want:.2}"), "absolute+keyframes"]);
        app.preview_sent = want;
        app.preview_at = Instant::now();
    }
    let has = PREVIEW_FRAME.with(|f| *f.borrow());
    if has {
        if let Some(ui) = app.ui() {
            if !ui.global::<Video>().get_has_preview() {
                ui.global::<Video>().set_has_preview(true);
            }
        }
    }
}

/// Barwa przewodnia płynnie podąża za kolorami obrazu.
fn tint_tick(app: &mut App) {
    let sample = SAMPLE.with(|s| s.borrow_mut().take()).filter(|_| picture_on(app));
    if let Some((buf, w, h)) = sample {
        if let Some(img) = image::RgbaImage::from_raw(w, h, buf) {
            // Szkło okien i menu w barwach filmu: próbka powiększona i rozmyta.
            // Odczyt z karty jest odwrócony w pionie.
            let gw = 128u32;
            let gh = ((gw as f32 * h as f32 / w.max(1) as f32).round() as u32).clamp(8, 256);
            let glass = image::imageops::blur(&image::imageops::flip_vertical(&image::imageops::resize(&img, gw, gh, image::imageops::FilterType::Triangle)), 6.0);
            if let Some(ui) = app.ui() {
                let th = ui.global::<Theme>();
                th.set_glass_image(slint::Image::from_rgba8(slint::SharedPixelBuffer::clone_from_slice(glass.as_raw(), glass.width(), glass.height())));
                th.set_glass_has(true);
            }
            let pal = crate::display::palette(&img, 3);
            if let Some(&(r, g, b)) = pal.first() {
                app.tint_target = Some(slint::Color::from_rgb_u8(r, g, b));
                if let Some(ui) = app.ui() {
                    let t = ui.global::<Theme>();
                    let c = |i: usize| pal.get(i).map(|&(r, g, b)| slint::Color::from_rgb_u8(r, g, b)).unwrap_or(slint::Color::from_rgb_u8(r, g, b));
                    t.set_grad_a(c(0));
                    t.set_grad_b(c(1));
                    t.set_grad_c(c(2));
                }
            }
        }
    }
    let on = app.s.video_tint && picture_on(app);
    let Some(target) = app.tint_target.filter(|_| on) else { return };
    let Some(ui) = app.ui() else { return };
    let t = ui.global::<Theme>();
    let cur = t.get_tint();
    let d = (cur.red() as i32 - target.red() as i32).abs() + (cur.green() as i32 - target.green() as i32).abs() + (cur.blue() as i32 - target.blue() as i32).abs();
    if d == 0 {
        return;
    }
    // Ostatnie kroki od razu – zaokrąglanie do bajtu zatrzymałoby przejście tuż przed celem.
    t.set_tint(if d <= 12 { target } else { cur.mix(&target, 0.9) });
}

/// Co klatkę: chowanie sterowania, komunikaty, podgląd, barwa; co pół sekundy – bufor i szczegóły.
pub fn tick(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    if let Some(t) = app.osd_until {
        if Instant::now() > t {
            ui.global::<Video>().set_osd_visible(false);
            app.osd_until = None;
        }
    }
    preview_tick(app);
    tint_tick(app);
    if app.video_slow.elapsed() > Duration::from_millis(500) {
        app.video_slow = Instant::now();
        slow_tick(app);
    }
    if ui.global::<Ui>().get_mode() != 1 {
        return;
    }
    let v = ui.global::<Video>();
    let fs = v.get_fullscreen();
    let wide = !fs;
    let idle = app.last_mouse.elapsed().as_millis() as i32 > app.s.autohide_ms;
    let may_hide = fs || app.s.autohide_window;
    let show = wide || !idle || !may_hide || app.hover_controls || v.get_panel() || !v.get_playing();
    if v.get_controls() != show {
        v.set_controls(show);
    }
    // W szerokim widoku kursor nad obrazem też znika po chwili.
    let cursor = !idle || !v.get_playing();
    if v.get_cursor() != cursor {
        v.set_cursor(cursor);
    }
}

/// Przycisk „Pomiń czołówkę / napisy końcowe”.
pub fn skip(app: &mut App) {
    let to = app.ui().map(|u| u.global::<Video>().get_skip_to()).unwrap_or(0.0);
    if to < 0.0 {
        crate::player::next(app);
    } else {
        crate::player::seek_abs(app, to as f64);
    }
}

/// Czołówka i napisy końcowe (z nazw rozdziałów) oraz odliczanie do następnego odcinka.
fn skip_tick(app: &mut App, pos: f64, dur: f64) {
    let Some(ui) = app.ui() else { return };
    let v = ui.global::<Video>();
    let is_video = app.cur.as_ref().is_some_and(|c| c.video);
    let (mut text, mut to) = (String::new(), 0.0f64);
    if is_video {
        if let Some(i) = app.chapters.iter().rposition(|c| c.0 <= pos + 0.5) {
            let n = app.chapters[i].1.to_lowercase();
            let has = |ks: &[&str]| ks.iter().any(|k| n.contains(k));
            if has(&["intro", "opening", "czołówk", "czolowk"]) || n == "op" {
                text = tr!("Skip intro").into();
                to = app.chapters.get(i + 1).map(|c| c.0).unwrap_or(dur);
            } else if has(&["credits", "ending", "outro", "napisy końcowe", "napisy koncowe"]) || n == "ed" {
                text = tr!("Skip credits").into();
                to = -1.0;
            }
        }
    }
    let (mut next, mut secs) = (String::new(), 0);
    if is_video && app.s.video_autonext && !app.next_cancelled && dur > 60.0 && dur - pos <= 20.0 {
        if let Some((l, i, _)) = crate::player::next_of(app, true, false) {
            if let Some(t) = crate::player::track_at(app, l, i).filter(|t| t.video || meta::is_video(std::path::Path::new(&t.path))) {
                next = t.display_title();
                secs = (dur - pos).ceil().max(0.0) as i32;
            }
        }
    }
    if v.get_skip_text().as_str() != text {
        v.set_skip_text(ss(&text));
    }
    v.set_skip_to(to as f32);
    if v.get_next_text().as_str() != next {
        v.set_next_text(ss(&next));
    }
    if v.get_next_secs() != secs {
        v.set_next_secs(secs);
    }
}

fn slow_tick(app: &mut App) {
    let Some(m) = app.mpv.clone() else { return };
    let Some(ui) = app.ui() else { return };
    if !app.cur.as_ref().is_some_and(|c| c.backend == Backend::Mpv && c.item != 0) {
        return;
    }
    let v = ui.global::<Video>();
    let (pos, dur) = crate::player::position(app);
    if dur > 0.0 {
        let cache = m.get_f64("demuxer-cache-time").unwrap_or(0.0);
        let b = if cache > pos { (cache / dur).clamp(0.0, 1.0) as f32 } else { 0.0 };
        if (b - v.get_buffered()).abs() > 0.002 {
            v.set_buffered(b);
        }
    }
    skip_tick(app, pos, dur);
    let wide = !v.get_fullscreen() && ui.global::<Ui>().get_mode() == 1;
    let visible = v.get_panel_page() == 4 && (v.get_panel() || (wide && v.get_side()));
    if visible {
        stats(app);
    }
}
