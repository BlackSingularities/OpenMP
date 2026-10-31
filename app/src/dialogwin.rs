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
    props!(v, Theme: dark, accent, tint);
    props!(v, Prefs:
        accent, accent_from_cover, audio_lang, author_line, auto_eq, autohide_ms, balance, check_updates,
        crossfade, crossfeed, dark, deinterlace, device, devices, eq, eq_on, folders, follow_playing,
        group_albums, hwdec, keys, limiter, media_keys, mono, mpv_info, mpv_ok, online_lyrics, preamp,
        preset, presets, prevent_sleep, remaining, replaygain, resume_on_start, rg_preamp,
        screenshot_dir, scroll_title, seek_step, show_sidebar, show_waveform, single_instance,
        soft_pause, speed, sub_lang, sub_size, swap, version, video_fit, video_folders, video_resume,
        viz_bands, viz_sensitivity, wave_style, width,
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
        add_network_folder(i), remove_folder(i, j), pick_screenshot_dir(), check_update(), open_url(u),
        associate(),
    );
    fwd!(Ui:
        url_accepted(t), tag_save(v), tag_pick_cover(), update_install(), update_skip(), rename_accepted(t),
    );
    let dw = d.as_weak();
    d.global::<Ui>().on_dialog_drag(move || {
        if let Some(d) = dw.upgrade() {
            d.window().with_winit_window(|w| {
                let _ = w.drag_window();
            });
        }
    });
    Some(Dialogs { win: d, props: all_props(), open: false })
}

/// Okno główne otworzyło albo zamknęło dialog.
pub fn shown(app: &mut App, on: bool) {
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
