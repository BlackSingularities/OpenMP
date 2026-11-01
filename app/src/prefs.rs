//! Ustawienia: lustro w interfejsie (`Prefs`), reakcja na zmiany, korektor i jego ustawienia
//! gotowe, foldery biblioteki (lokalne i sieciowe).

use openmp_core::audio::dsp;
use slint::{Color, Model, ModelRc, VecModel};

use crate::app::*;
use crate::*;

pub fn parse_color(s: &str) -> Color {
    let h = s.trim_start_matches('#');
    let v = u32::from_str_radix(h, 16).unwrap_or(0x4f8cff);
    Color::from_rgb_u8((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

pub fn presets(app: &App) -> Vec<(String, Vec<f32>)> {
    let mut v: Vec<(String, Vec<f32>)> = dsp::presets().into_iter().map(|(n, g)| (n.to_string(), g.to_vec())).collect();
    v.extend(app.s.custom_eq.iter().cloned());
    v
}

pub fn publish_presets(app: &mut App) {
    let p = presets(app);
    let mut names: Vec<String> = p.iter().map(|x| x.0.clone()).collect();
    let mut idx = names.iter().position(|n| *n == app.s.eq_preset);
    if idx.is_none() {
        names.push("Własne".into());
        idx = Some(names.len() - 1);
    }
    if let Some(ui) = app.ui() {
        let pr = ui.global::<Prefs>();
        pr.set_presets(crate::app::strings(&names));
        pr.set_preset(idx.unwrap_or(0) as i32);
    }
}

/// Wszystkie ustawienia do interfejsu (start programu).
pub fn publish(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    let s = app.s.clone();
    let t = ui.global::<Theme>();
    t.set_dark(s.theme != "light");
    t.set_accent(parse_color(&s.accent));
    t.set_tint(parse_color(&s.accent));
    let p = ui.global::<Prefs>();
    p.set_dark(s.theme != "light");
    p.set_accent(ss(&s.accent));
    p.set_accent_from_cover(s.accent_from_cover);
    p.set_viz_bands(s.viz_bands);
    p.set_viz_sensitivity(s.viz_sensitivity);
    p.set_show_waveform(s.show_waveform);
    p.set_auto_eq(s.auto_eq);
    p.set_wave_style(s.wave_style);
    p.set_online_lyrics(s.online_lyrics);
    p.set_online_covers(s.online_covers);
    openmp_core::coverart::set_enabled(s.online_covers);
    p.set_group_albums(s.group_albums);
    p.set_remaining(s.show_remaining);
    p.set_compact(s.compact);
    p.set_ontop(s.always_on_top);
    p.set_show_sidebar(s.show_sidebar);
    p.set_scroll_title(s.scroll_title);
    p.set_crossfade(s.crossfade as f32);
    p.set_soft_pause(s.soft_pause);
    p.set_exclusive(s.exclusive);
    p.set_resample_quality(s.resample_quality);
    p.set_replaygain(s.replaygain);
    p.set_rg_preamp(s.rg_preamp);
    p.set_speed(s.speed as f32);
    p.set_eq_on(s.dsp.eq_on);
    app.m.eq.set_vec(s.dsp.gains.clone());
    p.set_eq(model(&app.m.eq));
    p.set_preamp(s.dsp.preamp);
    p.set_balance(s.dsp.balance);
    p.set_width(s.dsp.width);
    p.set_mono(s.dsp.mono);
    p.set_swap(s.dsp.swap);
    p.set_crossfeed(s.dsp.crossfeed);
    p.set_limiter(s.dsp.limiter);
    p.set_resume_on_start(s.resume_on_start);
    p.set_follow_playing(s.follow_playing);
    p.set_seek_step(s.seek_step as f32);
    p.set_video_resume(s.video_resume);
    p.set_accent_scope(s.accent_scope);
    p.set_backdrop(s.backdrop);
    p.set_backdrop_opacity(s.backdrop_opacity);
    p.set_backdrop_motion(s.backdrop_motion);
    p.set_glow(s.glow);
    p.set_corners(s.corners);
    p.set_text_scale(s.text_scale);
    p.set_title_font(s.title_font);
    p.set_compact_ui(s.compact_ui);
    p.set_track_grid(s.track_grid);
    p.set_grid_size(s.grid_size);
    publish_theme(app);
    p.set_video_ui(s.video_ui);
    p.set_video_leave(s.video_leave);
    p.set_video_open_view(s.video_open_view);
    p.set_autohide_window(s.autohide_window);
    p.set_video_quality(s.video_quality);
    p.set_hwdec_mode(s.hwdec_mode);
    p.set_interpolation(s.interpolation);
    p.set_deband(s.deband);
    ui.global::<Video>().set_deband(s.deband);
    ui.global::<Video>().set_panel_page(if s.video_ui == 1 { 5 } else { 0 });
    p.set_tone_mapping(s.tone_mapping);
    p.set_ambient(s.ambient);
    p.set_video_tint(s.video_tint);
    p.set_seek_preview(s.seek_preview);
    p.set_wheel_action(s.wheel_action);
    p.set_dblclick_seek(s.dblclick_seek);
    p.set_video_autonext(s.video_autonext);
    p.set_volume_boost(s.volume_boost);
    p.set_night_mode(s.night_mode);
    p.set_downmix(s.downmix);
    p.set_ytdl_format(s.ytdl_format);
    p.set_ytdl(s.ytdl);
    p.set_cache_mb(s.cache_mb);
    p.set_sub_font(ss(&s.sub_font));
    p.set_sub_color(s.sub_color);
    p.set_sub_style(s.sub_style);
    p.set_sub_pos(s.sub_pos);
    p.set_sub_bold(s.sub_bold);
    p.set_sub_override(s.sub_override);
    p.set_sub_auto(s.sub_auto);
    p.set_autohide_ms(s.autohide_ms);
    p.set_video_fit(s.video_fit_window);
    p.set_sub_size(s.sub_size);
    p.set_sub_lang(ss(&s.sub_lang));
    p.set_lb_token(ss(&s.listenbrainz_token));
    p.set_audio_lang(ss(&s.audio_lang));
    p.set_screenshot_dir(ss(&s.screenshot_dir));
    p.set_deinterlace(s.deinterlace);
    p.set_single_instance(s.single_instance);
    p.set_media_keys(s.media_keys);
    p.set_global_hotkeys(s.global_hotkeys);
    p.set_prevent_sleep(s.prevent_sleep);
    p.set_check_updates(s.check_updates);
    p.set_folders(crate::app::strings(&s.library_folders));
    p.set_video_folders(crate::app::strings(&s.video_folders));
    p.set_version(ss(openmp_core::version()));
    p.set_author_line(ss(openmp_core::AUTHOR_LINE));
    let mut devs = vec!["Domyślne urządzenie systemu".to_string()];
    devs.extend(openmp_core::audio::engine::devices());
    let di = s.device.as_ref().and_then(|d| devs.iter().position(|x| x == d)).unwrap_or(0);
    p.set_devices(crate::app::strings(&devs));
    p.set_device(di as i32);
    p.set_mpv_ok(app.mpv.is_some());
    p.set_mpv_info(ss(if app.mpv.is_some() { "libmpv – wideo, napisy, strumienie, dekodowanie sprzętowe".to_string() } else { app.mpv_err.clone() }));
    p.set_keys(ModelRc::new(VecModel::from(crate::keys::help())));
    let pl = ui.global::<Player>();
    pl.set_shuffle(s.shuffle);
    pl.set_repeat(s.repeat);
    pl.set_viz_mode(s.viz);
    pl.set_volume(s.volume);
    pl.set_muted(s.muted);
    pl.set_speed_text(ss(speed_text(s.speed)));
    publish_presets(app);
}

/// Wygląd całego programu (zasięg akcentu, zaokrąglenia, wielkość tekstu, czcionka, poświata).
pub fn publish_theme(app: &App) {
    let Some(ui) = app.ui() else { return };
    let t = ui.global::<Theme>();
    t.set_scope(app.s.accent_scope.clamp(0, 2));
    t.set_corners(app.s.corners.clamp(0, 2));
    t.set_text_scale(app.s.text_scale.clamp(0.8, 1.4));
    t.set_title_font(app.s.title_font.clamp(0, 2));
    t.set_glow_k(app.s.glow.clamp(0.0, 2.0));
    t.set_dense(app.s.compact_ui);
}

/// Opcje mpv zależne od ustawienia `key` (po zmianie w oknie ustawień).
fn apply_mpv(app: &App, key: &str) {
    let Some(m) = &app.mpv else { return };
    match key {
        "video-quality" | "interpolation" | "deband" => crate::video::apply_quality(m, &app.s),
        "sub-font" | "sub-color" | "sub-style" | "sub-pos" | "sub-bold" | "sub-override" | "sub-auto" | "sub-size" => crate::video::apply_subs(m, &app.s),
        _ => crate::video::apply_all(m, &app.s),
    }
}

fn speed_text(s: f64) -> String {
    if (s - 1.0).abs() < 0.001 {
        String::new()
    } else {
        format!("{}×", format!("{s:.2}").trim_end_matches('0').trim_end_matches('.')).replace('.', ",")
    }
}

/// Silnik dźwięku według ustawień.
pub fn apply_audio(app: &mut App) {
    app.engine.set_dsp(app.s.dsp.clone());
    app.engine.set_crossfade(app.s.crossfade);
    app.engine.set_soft_pause(app.s.soft_pause);
    openmp_core::audio::decoder::set_resample_quality(app.s.resample_quality.max(0) as u32);
    app.engine.set_exclusive(app.s.exclusive);
    app.engine.set_speed(app.s.speed);
    crate::video::apply_audio_filters(app);
    crate::player::apply_volume(app);
}

/// Zmiana ustawienia w interfejsie.
pub fn changed(app: &mut App, key: &str) {
    let Some(ui) = app.ui() else { return };
    let p = ui.global::<Prefs>();
    match key {
        "dark" => {
            app.s.theme = if p.get_dark() { "dark".into() } else { "light".into() };
            ui.global::<Theme>().set_dark(p.get_dark());
        }
        "accent" => {
            // Własny kolor wpisywany ręcznie – dopiero pełny zapis #RRGGBB.
            let a = p.get_accent().trim().to_string();
            let a = if a.starts_with('#') { a } else { format!("#{a}") };
            if a.len() != 7 || u32::from_str_radix(&a[1..], 16).is_err() {
                return;
            }
            app.s.accent = a;
            let c = parse_color(&app.s.accent);
            ui.global::<Theme>().set_accent(c);
            if !app.s.accent_from_cover || app.cur.is_none() {
                ui.global::<Theme>().set_tint(c);
            }
        }
        "accent-from-cover" => {
            app.s.accent_from_cover = p.get_accent_from_cover();
            crate::display::show_track(app);
            if app.cur.is_none() {
                ui.global::<Theme>().set_tint(parse_color(&app.s.accent));
            }
        }
        "viz-bands" => app.s.viz_bands = p.get_viz_bands(),
        "viz-sensitivity" => app.s.viz_sensitivity = p.get_viz_sensitivity(),
        "show-waveform" => {
            app.s.show_waveform = p.get_show_waveform();
            if app.s.show_waveform {
                crate::display::show_track(app);
            }
        }
        "auto-eq" => {
            app.s.auto_eq = p.get_auto_eq();
            if app.s.auto_eq {
                auto_eq(app, true);
            }
        }
        "wave-style" => {
            app.s.wave_style = p.get_wave_style();
            crate::display::restyle_wave(app);
        }
        "online-lyrics" => {
            app.s.online_lyrics = p.get_online_lyrics();
            if app.s.online_lyrics && app.lyrics_text.is_empty() {
                crate::display::show_track(app);
            }
        }
        "group-albums" => {
            app.s.group_albums = p.get_group_albums();
            crate::lists::rebuild(app);
        }
        "remaining" => app.s.show_remaining = p.get_remaining(),
        "ontop" => {
            app.s.always_on_top = p.get_ontop();
            crate::win::set_ontop(app, app.s.always_on_top);
        }
        "show-sidebar" => app.s.show_sidebar = p.get_show_sidebar(),
        "side-panel" => {}
        "scroll-title" => app.s.scroll_title = p.get_scroll_title(),
        "crossfade" => {
            app.s.crossfade = (p.get_crossfade() as f64 * 2.0).round() / 2.0;
            app.engine.set_crossfade(app.s.crossfade);
            crate::player::refresh_next(app);
        }
        "exclusive" => {
            app.s.exclusive = p.get_exclusive();
            app.engine.set_exclusive(app.s.exclusive);
        }
        "resample-quality" => {
            app.s.resample_quality = p.get_resample_quality();
            openmp_core::audio::decoder::set_resample_quality(app.s.resample_quality.max(0) as u32);
        }
        "soft-pause" => {
            app.s.soft_pause = p.get_soft_pause();
            app.engine.set_soft_pause(app.s.soft_pause);
        }
        "replaygain" | "rg-preamp" => {
            app.s.replaygain = p.get_replaygain();
            app.s.rg_preamp = p.get_rg_preamp();
            app.toast("ReplayGain zadziała od następnego utworu");
        }
        "speed" => {
            app.s.speed = (p.get_speed() as f64 * 20.0).round() / 20.0;
            app.engine.set_speed(app.s.speed);
            ui.global::<Player>().set_speed_text(ss(speed_text(app.s.speed)));
        }
        "eq-on" | "preamp" | "balance" | "width" | "mono" | "swap" | "crossfeed" | "limiter" => {
            let d = &mut app.s.dsp;
            d.eq_on = p.get_eq_on();
            d.preamp = (p.get_preamp() * 2.0).round() / 2.0;
            d.balance = p.get_balance();
            d.width = p.get_width();
            d.mono = p.get_mono();
            d.swap = p.get_swap();
            d.crossfeed = p.get_crossfeed();
            d.limiter = p.get_limiter();
            app.engine.set_dsp(app.s.dsp.clone());
            crate::video::apply_audio_filters(app);
            ui.global::<Player>().set_eq_on(app.s.dsp.eq_on);
        }
        "device" => {
            let i = p.get_device();
            app.s.device = if i <= 0 { None } else { p.get_devices().row_data(i as usize).map(|s| s.to_string()) };
            app.engine.set_device(app.s.device.clone());
            if let Some(m) = &app.mpv {
                m.set("audio-device", "auto");
            }
        }
        "resume-on-start" => app.s.resume_on_start = p.get_resume_on_start(),
        "follow-playing" => app.s.follow_playing = p.get_follow_playing(),
        "seek-step" => app.s.seek_step = p.get_seek_step().round() as f64,
        "hwdec-mode" => {
            app.s.hwdec_mode = p.get_hwdec_mode().clamp(0, 5);
            app.s.hwdec = app.s.hwdec_mode != 0;
            apply_mpv(app, key);
        }
        "accent-scope" | "corners" | "text-scale" | "title-font" | "glow" => {
            app.s.accent_scope = p.get_accent_scope();
            app.s.corners = p.get_corners();
            app.s.text_scale = (p.get_text_scale() * 20.0).round() / 20.0;
            app.s.title_font = p.get_title_font();
            app.s.glow = p.get_glow();
            publish_theme(app);
        }
        "compact-ui" => {
            app.s.compact_ui = p.get_compact_ui();
            publish_theme(app);
        }
        "track-grid" => {
            app.s.track_grid = p.get_track_grid();
            crate::lists::rebuild(app);
            crate::libview::refresh_grid_covers(app);
        }
        "grid-size" => app.s.grid_size = (p.get_grid_size() / 10.0).round() * 10.0,
        "backdrop" => app.s.backdrop = p.get_backdrop(),
        "backdrop-opacity" => app.s.backdrop_opacity = p.get_backdrop_opacity(),
        "backdrop-motion" => app.s.backdrop_motion = p.get_backdrop_motion(),
        "video-ui" => crate::video::set_layout(app, p.get_video_ui()),
        "video-leave" => app.s.video_leave = p.get_video_leave(),
        "video-open-view" => app.s.video_open_view = p.get_video_open_view(),
        "autohide-window" => app.s.autohide_window = p.get_autohide_window(),
        "ambient" => app.s.ambient = p.get_ambient(),
        "video-tint" => {
            app.s.video_tint = p.get_video_tint();
            if !app.s.video_tint {
                crate::display::show_track(app);
            }
        }
        "seek-preview" => app.s.seek_preview = p.get_seek_preview(),
        "wheel-action" => app.s.wheel_action = p.get_wheel_action(),
        "dblclick-seek" => app.s.dblclick_seek = p.get_dblclick_seek(),
        "video-autonext" => app.s.video_autonext = p.get_video_autonext(),
        "video-quality" | "interpolation" | "deband" | "tone-mapping" | "volume-boost" | "downmix" | "ytdl-format" | "ytdl" | "cache-mb" => {
            app.s.video_quality = p.get_video_quality();
            app.s.interpolation = p.get_interpolation();
            app.s.deband = p.get_deband();
            ui.global::<Video>().set_deband(app.s.deband);
            app.s.tone_mapping = p.get_tone_mapping();
            app.s.volume_boost = p.get_volume_boost().round();
            app.s.downmix = p.get_downmix();
            app.s.ytdl_format = p.get_ytdl_format();
            app.s.ytdl = p.get_ytdl();
            app.s.cache_mb = p.get_cache_mb();
            apply_mpv(app, key);
            if key == "volume-boost" {
                crate::player::apply_volume(app);
            }
        }
        "night-mode" => {
            app.s.night_mode = p.get_night_mode();
            crate::video::apply_audio_filters(app);
        }
        "sub-font" | "sub-color" | "sub-style" | "sub-pos" | "sub-bold" | "sub-override" | "sub-auto" => {
            app.s.sub_font = p.get_sub_font().trim().to_string();
            if app.s.sub_font.is_empty() {
                app.s.sub_font = "Segoe UI Semibold".into();
            }
            app.s.sub_color = p.get_sub_color();
            app.s.sub_style = p.get_sub_style();
            app.s.sub_pos = p.get_sub_pos();
            app.s.sub_bold = p.get_sub_bold();
            app.s.sub_override = p.get_sub_override();
            app.s.sub_auto = p.get_sub_auto();
            apply_mpv(app, key);
        }
        "video-resume" => app.s.video_resume = p.get_video_resume(),
        "autohide-ms" => app.s.autohide_ms = p.get_autohide_ms(),
        "video-fit" => app.s.video_fit_window = p.get_video_fit(),
        "sub-size" => {
            app.s.sub_size = p.get_sub_size();
            apply_mpv(app, key);
        }
        "online-covers" => {
            app.s.online_covers = p.get_online_covers();
            openmp_core::coverart::set_enabled(app.s.online_covers);
        }
        "lb-token" => app.s.listenbrainz_token = p.get_lb_token().trim().to_string(),
        "sub-lang" => {
            app.s.sub_lang = p.get_sub_lang().to_string();
            if let Some(m) = &app.mpv {
                m.set("slang", &app.s.sub_lang);
            }
        }
        "audio-lang" => {
            app.s.audio_lang = p.get_audio_lang().to_string();
            if let Some(m) = &app.mpv {
                m.set("alang", &app.s.audio_lang);
            }
        }
        "deinterlace" => {
            app.s.deinterlace = p.get_deinterlace();
            if let Some(m) = &app.mpv {
                m.set("deinterlace", if app.s.deinterlace { "yes" } else { "no" });
            }
        }
        "single-instance" => app.s.single_instance = p.get_single_instance(),
        "global-hotkeys" => {
            app.s.global_hotkeys = p.get_global_hotkeys();
            let n = crate::win::hotkeys(app, app.s.global_hotkeys);
            if app.s.global_hotkeys && n < 5 {
                app.toast(format!("Część skrótów zajmuje inny program ({n} z 5 działa)"));
            }
        }
        "media-keys" => {
            app.s.media_keys = p.get_media_keys();
            if app.s.media_keys {
                crate::win::smtc_init(app);
            } else {
                app.smtc = None;
            }
        }
        "prevent-sleep" => app.s.prevent_sleep = p.get_prevent_sleep(),
        "check-updates" => app.s.check_updates = p.get_check_updates(),
        "rescan" => crate::libview::start_scan(app),
        _ => {}
    }
    app.s.save();
}

pub fn eq_band(app: &mut App, i: usize, v: f32) {
    if i >= app.s.dsp.gains.len() {
        return;
    }
    app.s.dsp.gains[i] = v;
    app.m.eq.set_row_data(i, v);
    if !app.s.dsp.eq_on {
        app.s.dsp.eq_on = true;
        if let Some(ui) = app.ui() {
            ui.global::<Prefs>().set_eq_on(true);
            ui.global::<Player>().set_eq_on(true);
        }
    }
    app.s.eq_preset = "Własne".into();
    app.engine.set_dsp(app.s.dsp.clone());
    crate::video::apply_audio_filters(app);
    publish_presets(app);
    app.dirty = true;
}

pub fn eq_preset(app: &mut App, i: usize) {
    let p = presets(app);
    let Some((name, gains)) = p.get(i).cloned() else { return };
    app.s.dsp.gains = gains.clone();
    app.s.dsp.eq_on = true;
    app.s.eq_preset = name;
    app.m.eq.set_vec(gains);
    if let Some(ui) = app.ui() {
        ui.global::<Prefs>().set_eq_on(true);
        ui.global::<Player>().set_eq_on(true);
    }
    app.engine.set_dsp(app.s.dsp.clone());
    crate::video::apply_audio_filters(app);
    publish_presets(app);
    app.s.save();
}

/// Korektor dopasowany do gatunku bieżącego utworu (gdy włączony i gatunek jest rozpoznany).
pub fn auto_eq(app: &mut App, announce: bool) {
    if !app.s.auto_eq {
        return;
    }
    let Some(c) = &app.cur else { return };
    if c.video {
        return;
    }
    let Some(name) = dsp::preset_for_genre(&c.info.genre) else { return };
    if app.s.eq_preset == name && app.s.dsp.eq_on {
        return;
    }
    let Some(i) = presets(app).iter().position(|(n, _)| n == name) else { return };
    eq_preset(app, i);
    if announce {
        app.toast(format!("Korektor: {name} (gatunek {})", app.cur.as_ref().map(|c| c.info.genre.clone()).unwrap_or_default()));
    }
}

pub fn eq_reset(app: &mut App) {
    app.s.dsp.gains = vec![0.0; dsp::BANDS.len()];
    app.s.dsp.preamp = 0.0;
    app.s.eq_preset = "Płasko".into();
    app.m.eq.set_vec(app.s.dsp.gains.clone());
    if let Some(ui) = app.ui() {
        ui.global::<Prefs>().set_preamp(0.0);
    }
    app.engine.set_dsp(app.s.dsp.clone());
    crate::video::apply_audio_filters(app);
    publish_presets(app);
    app.s.save();
}

/// Dodanie folderu biblioteki: 0 – muzyka, 1 – wideo.
pub fn add_folder(app: &mut App, kind: i32) {
    let title = if kind == 1 { "Folder z wideo" } else { "Folder z muzyką" };
    crate::dialogs::ask(app, move |d| d.set_title(title).pick_folder(), move |app, dir| {
        add_folder_path(app, kind, dir.to_string_lossy().into_owned());
    });
}

pub fn add_network_folder(app: &mut App, kind: i32) {
    crate::dialogs::rename(app, RenameCtx::NetFolder(kind), "Ścieżka sieciowa (np. \\\\NAS\\Muzyka lub Z:\\Filmy)", "\\\\");
}

pub fn add_folder_path(app: &mut App, kind: i32, path: String) {
    let path = path.trim().trim_end_matches(['\\', '/']).to_string();
    if path.len() < 3 {
        return;
    }
    if !openmp_core::library::reachable(&path) {
        app.toast(format!("Folder „{path}” jest niedostępny – sprawdź ścieżkę i połączenie z siecią"));
        return;
    }
    let list = if kind == 1 { &mut app.s.video_folders } else { &mut app.s.library_folders };
    if list.iter().any(|f| f.eq_ignore_ascii_case(&path)) {
        return;
    }
    list.push(path);
    app.s.save();
    publish_folders(app);
    crate::libview::start_scan(app);
}

pub fn remove_folder(app: &mut App, kind: i32, i: usize) {
    let list = if kind == 1 { &mut app.s.video_folders } else { &mut app.s.library_folders };
    if i < list.len() {
        let gone = list.remove(i).to_lowercase();
        // Pliki z usuniętego folderu znikają od razu, bez czekania na skan.
        let target = if kind == 1 { &mut app.lib.videos } else { &mut app.lib.tracks };
        target.retain(|t| !t.path.to_lowercase().starts_with(&gone));
        app.lib.save();
    }
    app.s.save();
    publish_folders(app);
    crate::libview::rebuild_nav(app);
}

fn publish_folders(app: &mut App) {
    crate::libview::watch(app);
    if let Some(ui) = app.ui() {
        ui.global::<Prefs>().set_folders(crate::app::strings(&app.s.library_folders));
        ui.global::<Prefs>().set_video_folders(crate::app::strings(&app.s.video_folders));
    }
}

pub fn pick_screenshot_dir(app: &mut App) {
    crate::dialogs::ask(app, |d| d.set_title("Folder zrzutów ekranu").pick_folder(), |app, d| {
        app.s.screenshot_dir = d.to_string_lossy().into_owned();
        if let Some(ui) = app.ui() {
            ui.global::<Prefs>().set_screenshot_dir(ss(&app.s.screenshot_dir));
        }
        app.s.save();
    });
}
