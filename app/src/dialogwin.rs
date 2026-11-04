//! Okna dialogowe (ustawienia, korektor, edytor tagów…) jako osobne okno, które może wystawać poza
//! okno programu. Okno ma własne kopie stanu Slint (Prefs, Ui, Theme), więc co klatkę
//! synchronizujemy je z oknem głównym w obie strony: zmiana w dialogu trafia do programu, zmiana
//! z programu (postęp aktualizacji, nowe presety) – do dialogu.

use slint::winit_030::WinitWindowAccessor;
use slint::{ComponentHandle, PhysicalPosition};

use crate::app::*;
use crate::*;

pub trait SyncProp {
    fn tick(&mut self, m: &AppWindow, d: &DialogWindow);
    fn reset(&mut self, m: &AppWindow, d: &DialogWindow);
}

struct Prop<T> {
    last: Option<T>,
    gm: fn(&AppWindow) -> T,
    sm: fn(&AppWindow, T),
    gd: fn(&DialogWindow) -> T,
    sd: fn(&DialogWindow, T),
}

impl<T: PartialEq + Clone> SyncProp for Prop<T> {
    fn tick(&mut self, m: &AppWindow, d: &DialogWindow) {
        let dv = (self.gd)(d);
        if self.last.as_ref() != Some(&dv) {
            (self.sm)(m, dv.clone());
            self.last = Some(dv);
            return;
        }
        let mv = (self.gm)(m);
        if self.last.as_ref() != Some(&mv) {
            (self.sd)(d, mv.clone());
            self.last = Some(mv);
        }
    }
    fn reset(&mut self, m: &AppWindow, d: &DialogWindow) {
        let mv = (self.gm)(m);
        (self.sd)(d, mv.clone());
        self.last = Some(mv);
    }
}

macro_rules! props {
    ($v:ident, $g:ident: $($p:ident),* $(,)?) => {
        paste::paste! {
            $(
                $v.push(Box::new(Prop {
                    last: None,
                    gm: |m: &AppWindow| m.global::<$g>().[<get_ $p>](),
                    sm: |m: &AppWindow, x| m.global::<$g>().[<set_ $p>](x),
                    gd: |d: &DialogWindow| d.global::<$g>().[<get_ $p>](),
                    sd: |d: &DialogWindow, x| d.global::<$g>().[<set_ $p>](x),
                }) as Box<dyn SyncProp>);
            )*
        }
    };
}

fn all_props() -> Vec<Box<dyn SyncProp>> {
    let mut v: Vec<Box<dyn SyncProp>> = Vec::new();
    props!(v, Theme: dark, accent, tint, scope, corners, text_scale, title_font, glow_k, dense, glass, nav_scale, glass_tint, glass_image, glass_has, cover_has, cover_color);
    props!(v, Prefs:
        compact_ui, track_grid, grid_size, caption_overlay, tray, custom_keys, accent_scope, backdrop, backdrop_opacity, backdrop_motion, glow, corners, text_scale, title_font,
        video_leave, video_open_view, autohide_window, video_quality, hwdec_mode, interpolation,
        deband, tone_mapping, ambient, video_tint, tint_rate, seek_preview, wheel_action, dblclick_seek, video_autonext,
        volume_boost, night_mode, downmix, ytdl_format, ytdl, cache_mb, sub_font, sub_color, sub_style,
        sub_pos, sub_bold, sub_override, sub_auto,
    );
    props!(v, Prefs:
        accent, accent_from_cover, languages, language, audio_lang, author_line, auto_eq, autohide_ms, balance, check_updates,
        crossfade, crossfeed, dark, deinterlace, device, devices, eq, eq_on, folders, follow_playing,
        group_albums, keys, limiter, media_keys, global_hotkeys, mono, mpv_info, mpv_ok, online_lyrics, preamp,
        lb_token, online_covers, preset, presets, prevent_sleep, remaining, replaygain, resample_quality, exclusive, resume_on_start, rg_preamp,
        screenshot_dir, scroll_title, seek_step, show_sidebar, show_waveform, single_instance,
        soft_pause, speed, sub_lang, sub_size, swap, version, video_folders, video_resume,
        viz_bands, viz_sensitivity, viz_color, viz_dim, viz_blur, viz_fps, viz_speed, viz_place, wave_style, width,
    );
    props!(v, Player: viz_mode);
    props!(v, Prefs:
        nav_playlist, nav_music, nav_video, nav_sets, nav_smart, nav_folders, sidebar_width, panel_glass, glass_tint, nav_text,
        live_style, live_span, live_glow, tmdb_key, os_key, peq_on, peq_name,
        ai_provider, ai_key, ai_url, ai_model, ai_models, ai_temperature, ai_status, ai_count, ai_busy,
    );
    props!(v, Prefs:
        bass, treble, eq_curve, eq_curve_fill, eq_peak, preset_custom,
        lyr_font, lyr_size, lyr_active_scale, lyr_spacing, lyr_color, lyr_active_color, lyr_bold, lyr_fade, lyr_align,
    );
    props!(v, Ui:
        about, equalizer, rename_dialog, rename_text, rename_title, settings, settings_page, tag_busy,
        tag_cover, tag_editor, tag_has_cover, tag_mixed, tag_title, tag_values, update_dialog,
        update_notes, update_portable, update_progress, update_status, update_version, url_dialog,
        url_text,
    );
    v
}

/// Stan okna dialogów trzymany w App.
pub struct Dialogs {
    pub win: DialogWindow,
    props: Vec<Box<dyn SyncProp>>,
    pub open: bool,
}

fn create(main: &AppWindow) -> Option<Dialogs> {
    let d = DialogWindow::new().ok()?;
    // Wywołania z dialogu wykonuje okno główne – po wcześniejszym przeniesieniu zmian.
    macro_rules! fwd {
        ($g:ident: $($cb:ident($($a:ident),*)),* $(,)?) => {
            paste::paste! {
                $(
                    let w = main.as_weak();
                    d.global::<$g>().[<on_ $cb>](move |$($a),*| {
                        later(sync_now);
                        if let Some(m) = w.upgrade() {
                            m.global::<$g>().[<invoke_ $cb>]($($a),*);
                        }
                    });
                )*
            }
        };
    }
    fwd!(Prefs:
        changed(k), eq_band(i, v), eq_preset(i), eq_save_preset(), eq_reset(), add_folder(i),
        add_network_folder(i), remove_folder(i, j), pick_screenshot_dir(), backup_export(), backup_import(), custom_key(i, t, c, s, a), check_update(), open_url(u),
        associate(), ai_fetch_models(), ai_analyze(a), ai_stop(), ai_forget(),
    );
    fwd!(Player: set_viz(m));
    fwd!(Ui:
        url_accepted(t), tag_save(v), tag_pick_cover(), update_install(), update_skip(), rename_accepted(t),
    );
    let mw = d.as_weak();
    d.global::<Ui>().on_dialog_move(move |dx, dy| {
        if let Some(d) = mw.upgrade() {
            let w = d.window();
            let sf = w.scale_factor();
            let p = w.position();
            w.set_position(slint::PhysicalPosition::new(p.x + (dx * sf).round() as i32, p.y + (dy * sf).round() as i32));
        }
    });
    let dw = d.as_weak();
    d.global::<Ui>().on_dialog_drag(move || {
        if let Some(d) = dw.upgrade() {
            d.window().with_winit_window(|w| {
                let _ = w.drag_window();
            });
        }
    });
    let sw = d.as_weak();
    d.global::<Ui>().on_settings_search(move |q| {
        let Some(d) = sw.upgrade() else { return };
        let rows: Vec<Opt> = settings_search(&q)
            .into_iter()
            .map(|(page, name, title)| Opt { id: ss(page.to_string()), label: ss(title), sub: ss(name), selected: false })
            .collect();
        d.global::<Ui>().set_settings_results(slint::ModelRc::new(slint::VecModel::from(rows)));
        if q.trim().is_empty() {
            d.global::<Theme>().set_settings_flash(ss(""));
        }
    });
    Some(Dialogs { win: d, props: all_props(), open: false })
}

/// Ustawienia do wyszukiwarki: (strona, nazwa strony, tytuł, opis) – czytane z pliku okna
/// ustawień przy kompilacji, więc każde nowe ustawienie od razu da się znaleźć. Teksty są
/// tłumaczone przy każdym wyszukiwaniu, więc pasują do bieżącego języka.
fn settings_index() -> Vec<(i32, String, String, String)> {
    {
        let src: &'static str = include_str!("../ui/dialogs.slint");
        let quoted = |l: &'static str| -> String {
            // Tekst ze wszystkich "…" w linii (opisy bywają składane z wyrażeń), przetłumaczony.
            l.split('"').skip(1).step_by(2).map(|q| openmp_core::i18n::tr(q).replace("{}", "")).collect::<Vec<_>>().join("")
        };
        let mut v: Vec<(i32, String, String, String)> = Vec::new();
        let (mut page, mut name) = (-1, String::new());
        let mut in_settings = false;
        for l in src.lines() {
            let t = l.trim();
            if t.starts_with("export component SettingsDialog") {
                in_settings = true;
            } else if t.starts_with("export component") {
                in_settings = false;
            }
            if !in_settings {
                continue;
            }
            if let Some(rest) = t.strip_prefix("if Ui.settings-page == ") {
                if let Some(n) = rest.split(':').next().and_then(|n| n.trim().parse().ok()) {
                    page = n;
                    name.clear();
                }
            } else if page >= 0 && t.starts_with("heading:") && name.is_empty() {
                name = quoted(t);
            } else if page >= 0 && t.starts_with("title:") {
                let title = quoted(t);
                if !title.is_empty() {
                    v.push((page, name.clone(), title, String::new()));
                }
            } else if page >= 0 && t.starts_with("hint:") {
                if let Some(last) = v.last_mut() {
                    if last.3.is_empty() {
                        last.3 = quoted(t);
                    }
                }
            }
        }
        v
    }
}

fn settings_search(q: &str) -> Vec<(i32, String, String)> {
    let q = q.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    settings_index()
        .into_iter()
        .filter(|(_, _, t, h)| t.to_lowercase().contains(&q) || h.to_lowercase().contains(&q))
        .take(40)
        .map(|(p, n, t, _)| (p, n, t))
        .collect()
}

/// Okno główne otworzyło albo zamknęło dialog.
pub fn shown(app: &mut App, on: bool) {
    // Dialog (np. o aktualizacji przy starcie) przy zminimalizowanym albo schowanym do zasobnika
    // oknie programu lądował poza ekranem – kliknięcie przywracało samo okno, bez dialogu.
    if on && !app.dialogs.as_ref().is_some_and(|d| d.open) {
        crate::win::raise(app);
    }
    let Some(ui) = app.ui() else { return };
    if app.dialogs.is_none() {
        app.dialogs = create(&ui);
    }
    let Some(dl) = &mut app.dialogs else { return };
    if on {
        for p in dl.props.iter_mut() {
            p.reset(&ui, &dl.win);
        }
        if !dl.open {
            dl.open = true;
            let _ = dl.win.show();
            crate::win::popup_style(dl.win.window());
            crate::win::acrylic(crate::win::hwnd_of(dl.win.window()), app.s.theme != "light");
            // Na środku okna programu – gdy okno ma już rozmiar aktywnego dialogu.
            let dw = dl.win.as_weak();
            let mw = ui.as_weak();
            slint::Timer::single_shot(std::time::Duration::from_millis(40), move || {
                let (Some(d), Some(m)) = (dw.upgrade(), mw.upgrade()) else { return };
                let (x, y, w, h) = crate::win::geometry(&m);
                let s = d.window().size();
                let px = x + (w as i32 - s.width as i32) / 2;
                let py = y + (h as i32 - s.height as i32) / 2;
                d.window().set_position(PhysicalPosition::new(px, py.max(0)));
            });
            dl.win.window().with_winit_window(|w| w.focus_window());
        }
    } else if dl.open {
        dl.open = false;
        let _ = dl.win.hide();
        ui.window().with_winit_window(|w| w.focus_window());
    }
}

/// Synchronizacja co klatkę, gdy dialog jest otwarty.
pub fn sync_now(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    let Some(dl) = &mut app.dialogs else { return };
    if !dl.open {
        return;
    }
    for p in dl.props.iter_mut() {
        p.tick(&ui, &dl.win);
    }
}
