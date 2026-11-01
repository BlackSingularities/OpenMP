//! Silnik muzyki: wątek dekodujący (Symphonia → przepróbkowanie → ReplayGain → przenikanie)
//! wypełnia bufor, a wywołanie zwrotne WASAPI (cpal) robi korektor, głośność, płynną pauzę
//! i oddaje próbki do wizualizacji.
//!
//! Każdy utwór ma identyfikator nadany przez aplikację; znaczniki w buforze mówią, od której
//! ramki gra który utwór – wyświetlacz przełącza się dokładnie wtedy, gdy słychać zmianę.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use crossbeam_channel::{Receiver, Sender};
use parking_lot::Mutex;

use super::decoder::{Resample, Source, StreamInfo};
use super::dsp::{Dsp, DspSettings};

/// Ile dźwięku trzymamy przed kartą (krótko – przewijanie i zmiana utworu są natychmiastowe).
const AHEAD_SECS: f64 = 0.3;
/// Długość wygaszenia przy przełączaniu i przewijaniu (bez trzasków).
const DECLICK_SECS: f64 = 0.03;

#[derive(Clone, Debug)]
pub struct Mark {
    pub frame: u64,
    pub id: u64,
    pub path: PathBuf,
    pub info: StreamInfo,
    pub base: f64,
    pub speed: f64,
}

struct Ring {
    buf: VecDeque<f32>,
    played: u64,
    written: u64,
    marks: VecDeque<Mark>,
    cur: Option<Mark>,
    end_frame: Option<u64>,
}

impl Ring {
    /// Obcina bufor do krótkiego wygaszenia – nowy dźwięk zaczyna się zaraz po nim.
    fn cut(&mut self, rate: u32) {
        let keep = ((rate as f64 * DECLICK_SECS) as usize * 2).min(self.buf.len());
        self.buf.truncate(keep);
        let n = keep / 2;
        for i in 0..n {
            let g = 1.0 - (i as f32 + 1.0) / n as f32;
            self.buf[i * 2] *= g;
            self.buf[i * 2 + 1] *= g;
        }
        self.written = self.played + n as u64;
        // Znaczniki z przyszłości tracą ważność.
        let w = self.written;
        self.marks.retain(|m| m.frame <= w);
        self.end_frame = None;
    }
}

/// Ostatnie próbki do wizualizacji (stereo, po korektorze, przed głośnością).
pub struct Scope {
    pub data: Vec<f32>,
    pub pos: usize,
    pub rate: u32,
    pub total: u64,
}

pub const SCOPE_FRAMES: usize = 8192;

impl Scope {
    fn push(&mut self, s: &[f32]) {
        for f in s.chunks_exact(2) {
            self.data[self.pos * 2] = f[0];
            self.data[self.pos * 2 + 1] = f[1];
            self.pos = (self.pos + 1) % SCOPE_FRAMES;
        }
        self.total += (s.len() / 2) as u64;
    }

    /// Ostatnie `n` ramek, od najstarszej.
    pub fn latest(&self, n: usize, out: &mut Vec<f32>) {
        out.clear();
        let n = n.min(SCOPE_FRAMES);
        let start = (self.pos + SCOPE_FRAMES - n) % SCOPE_FRAMES;
        for i in 0..n {
            let p = (start + i) % SCOPE_FRAMES;
            out.push(self.data[p * 2]);
            out.push(self.data[p * 2 + 1]);
        }
    }
}

pub(super) struct Shared {
    ring: Mutex<Ring>,
    vol: AtomicU32,
    paused: AtomicBool,
    soft: AtomicBool,
    dsp: Mutex<Dsp>,
    scope: Mutex<Scope>,
    rate: AtomicU32,
    pub(super) failed: AtomicBool,
    /// Bity wyjścia (0 – zmiennoprzecinkowe przez mikser Windows) i tryb wyłączny.
    out_bits: AtomicU32,
    exclusive: AtomicBool,
    /// Głośność ustawia sesja w mikserze Windows – próbki idą bez ściszania.
    session: AtomicBool,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub id: u64,
    pub path: PathBuf,
    /// ReplayGain (liniowo, z przedwzmocnieniem i ochroną przed przesterowaniem).
    pub gain: f32,
}

enum Cmd {
    Load(Item, f64),
    Next(Option<Item>),
    Seek(f64),
    Stop,
    Speed(f64),
    Crossfade(f64),
    Device(Option<String>),
    Loopback(bool),
    Exclusive(bool),
    Gain(u64, f32),
}

#[derive(Clone, Debug)]
pub enum Event {
    Error { id: u64, path: PathBuf, msg: String },
    Device(String),
    /// Głośność zmieniona w mikserze Windows (0..1, ta sama skala co `set_volume`).
    Volume(f32),
}

/// Stan do wyświetlacza.
#[derive(Clone, Debug, Default)]
pub struct Status {
    pub id: Option<u64>,
    pub path: Option<PathBuf>,
    pub info: StreamInfo,
    pub pos: f64,
    pub ended: bool,
    pub paused: bool,
}

pub struct Engine {
    tx: Sender<Cmd>,
    shared: Arc<Shared>,
    pub events: Receiver<Event>,
}

/// Nazwy urządzeń wyjściowych (pierwsza pozycja „domyślne” to `None` w aplikacji).
pub fn devices() -> Vec<String> {
    let host = cpal::default_host();
    host.output_devices().map(|d| d.filter_map(|d| d.name().ok()).collect()).unwrap_or_default()
}

impl Engine {
    pub fn new(device: Option<String>) -> Engine {
        let (tx, rx) = crossbeam_channel::unbounded();
        let (etx, erx) = crossbeam_channel::unbounded();
        let shared = Arc::new(Shared {
            ring: Mutex::new(Ring { buf: VecDeque::new(), played: 0, written: 0, marks: VecDeque::new(), cur: None, end_frame: None }),
            vol: AtomicU32::new(0.8f32.to_bits()),
            paused: AtomicBool::new(true),
            soft: AtomicBool::new(true),
            dsp: Mutex::new(Dsp::new(48000)),
            scope: Mutex::new(Scope { data: vec![0.0; SCOPE_FRAMES * 2], pos: 0, rate: 48000, total: 0 }),
            rate: AtomicU32::new(48000),
            failed: AtomicBool::new(false),
            out_bits: AtomicU32::new(0),
            exclusive: AtomicBool::new(false),
            session: AtomicBool::new(false),
        });
        let sh = shared.clone();
        std::thread::Builder::new()
            .name("openmp-audio".into())
            .spawn(move || Worker::new(sh, rx, etx, device).run())
            .expect("wątek audio");
        Engine { tx, shared, events: erx }
    }

    /// Wczytuje utwór i zaczyna od `start` sekund (pauza zostaje, jak była).
    pub fn load(&self, item: Item, start: f64) {
        let _ = self.tx.send(Cmd::Load(item, start));
    }
    /// Następny utwór – do przejścia bez przerwy albo przenikania.
    pub fn set_next(&self, item: Option<Item>) {
        let _ = self.tx.send(Cmd::Next(item));
    }
    pub fn seek(&self, secs: f64) {
        let _ = self.tx.send(Cmd::Seek(secs));
    }
    pub fn stop(&self) {
        self.shared.paused.store(true, Ordering::Relaxed);
        let _ = self.tx.send(Cmd::Stop);
    }
    pub fn set_paused(&self, p: bool) {
        self.shared.paused.store(p, Ordering::Relaxed);
    }
    pub fn paused(&self) -> bool {
        self.shared.paused.load(Ordering::Relaxed)
    }
    /// Głośność 0..1 (krzywa percepcyjna liczona tutaj; 100% = sygnał bez ściszania).
    pub fn set_volume(&self, v: f32) {
        let v = v.clamp(0.0, 1.0);
        let g = if v <= 0.0 { 0.0 } else { v * v.sqrt() };
        self.shared.vol.store(g.to_bits(), Ordering::Relaxed);
    }
    /// Czy głośność aplikacji idzie przez sesję w mikserze Windows.
    pub fn session_volume(&self) -> bool {
        self.shared.session.load(Ordering::Relaxed)
    }
    pub fn set_soft_pause(&self, on: bool) {
        self.shared.soft.store(on, Ordering::Relaxed);
    }
    pub fn set_speed(&self, s: f64) {
        let _ = self.tx.send(Cmd::Speed(s));
    }
    pub fn set_crossfade(&self, secs: f64) {
        let _ = self.tx.send(Cmd::Crossfade(secs));
    }
    pub fn set_device(&self, name: Option<String>) {
        let _ = self.tx.send(Cmd::Device(name));
    }
    /// Wizualizacja z dźwięku systemu (gdy gra mpv – wideo, radio, nietypowe formaty).
    pub fn set_loopback(&self, on: bool) {
        let _ = self.tx.send(Cmd::Loopback(on));
    }
    /// Nowe wzmocnienie utworu `id` (pomiar głośności skończył się w trakcie grania).
    pub fn set_gain(&self, id: u64, gain: f32) {
        let _ = self.tx.send(Cmd::Gain(id, gain));
    }
    /// Tryb wyłączny WASAPI: karta gra w częstotliwości pliku, z pominięciem miksera Windows.
    pub fn set_exclusive(&self, on: bool) {
        let _ = self.tx.send(Cmd::Exclusive(on));
    }
    /// Wyjście: częstotliwość, bity (0 – mikser Windows, f32) i czy działa tryb wyłączny.
    pub fn output(&self) -> (u32, u32, bool) {
        (self.rate(), self.shared.out_bits.load(Ordering::Relaxed), self.shared.exclusive.load(Ordering::Relaxed))
    }
    pub fn set_dsp(&self, s: DspSettings) {
        self.shared.dsp.lock().apply(s);
    }
    pub fn rate(&self) -> u32 {
        self.shared.rate.load(Ordering::Relaxed)
    }

    pub fn scope(&self, frames: usize, out: &mut Vec<f32>) -> u64 {
        let s = self.shared.scope.lock();
        s.latest(frames, out);
        s.total
    }

    pub fn status(&self) -> Status {
        let rate = self.rate() as f64;
        let mut r = self.shared.ring.lock();
        while r.marks.front().is_some_and(|m| m.frame <= r.played) {
            r.cur = r.marks.pop_front();
        }
        let ended = r.end_frame.is_some_and(|f| r.played >= f);
        let paused = self.paused();
        match &r.cur {
            Some(m) => Status {
                id: Some(m.id),
                path: Some(m.path.clone()),
                info: m.info.clone(),
                pos: m.base + (r.played.saturating_sub(m.frame)) as f64 / rate * m.speed,
                ended,
                paused,
            },
            None => Status { ended, paused, ..Default::default() },
        }
    }
}

struct Track {
    id: u64,
    path: PathBuf,
    src: Source,
    rs: Resample,
    pend: VecDeque<f32>,
    gain: f32,
    eof: bool,
    fade_in: usize,
    tmp: Vec<f32>,
    out: Vec<f32>,
}

impl Track {
    fn open(it: &Item, rate: u32, speed: f64) -> Result<Track, String> {
        let src = Source::open(&it.path)?;
        let rs = Resample::new(src.info.rate, rate, speed);
        Ok(Track {
            id: it.id,
            path: it.path.clone(),
            src,
            rs,
            pend: VecDeque::new(),
            gain: it.gain,
            eof: false,
            fade_in: (rate as f64 * DECLICK_SECS) as usize,
            tmp: Vec::new(),
            out: Vec::new(),
        })
    }

    fn fill(&mut self, frames: usize) {
        while !self.eof && self.pend.len() < frames * 2 {
            self.tmp.clear();
            self.out.clear();
            if !self.src.read(&mut self.tmp) {
                self.rs.flush(&mut self.out);
                self.eof = true;
            } else {
                if (self.gain - 1.0).abs() > 1e-4 {
                    for s in &mut self.tmp {
                        *s *= self.gain;
                    }
                }
                self.rs.process(&self.tmp, &mut self.out);
            }
            self.pend.extend(self.out.iter().copied());
        }
    }

    fn take(&mut self) -> Option<(f32, f32)> {
        let l = self.pend.pop_front()?;
        let r = self.pend.pop_front().unwrap_or(l);
        if self.fade_in > 0 {
            self.fade_in -= 1;
        }
        Some((l, r))
    }

    /// Sekundy, które jeszcze nie trafiły do bufora wyjścia.
    fn remaining(&self, rate: u32) -> f64 {
        if self.src.info.duration <= 0.0 {
            return f64::INFINITY;
        }
        (self.src.info.duration - self.src.time).max(0.0) + self.pend.len() as f64 / 2.0 / rate as f64
    }

    fn mark(&self, frame: u64, base: f64, speed: f64) -> Mark {
        Mark { frame, id: self.id, path: self.path.clone(), info: self.src.info.clone(), base, speed }
    }
}

struct Worker {
    sh: Arc<Shared>,
    rx: Receiver<Cmd>,
    etx: Sender<Event>,
    device: Option<String>,
    stream: Option<cpal::Stream>,
    excl: Option<super::exclusive::Exclusive>,
    /// Sesja w mikserze Windows i ostatnio ustawiona na niej głośność.
    session: Option<windows::Win32::Media::Audio::ISimpleAudioVolume>,
    session_set: f32,
    want_excl: bool,
    loopback: Option<cpal::Stream>,
    want_loopback: bool,
    rate: u32,
    cur: Option<Track>,
    next: Option<Track>,
    /// Następny utwór czekający na przełączenie karty (tryb wyłączny, inna częstotliwość).
    switch: Option<Track>,
    next_item: Option<Item>,
    fade: Option<(usize, usize)>,
    crossfade: f64,
    speed: f64,
    chunk: Vec<f32>,
}

impl Worker {
    fn new(sh: Arc<Shared>, rx: Receiver<Cmd>, etx: Sender<Event>, device: Option<String>) -> Worker {
        let mut w = Worker {
            sh,
            rx,
            etx,
            device,
            stream: None,
            excl: None,
            session: None,
            session_set: -1.0,
            want_excl: false,
            loopback: None,
            want_loopback: false,
            rate: 48000,
            cur: None,
            next: None,
            switch: None,
            next_item: None,
            fade: None,
            crossfade: 0.0,
            speed: 1.0,
            chunk: Vec::new(),
        };
        w.open_stream();
        w
    }

    /// Nowa częstotliwość wyjścia: korektor, wizualizacja i przepróbkowanie utworów.
    fn set_rate(&mut self, rate: u32, reseek: bool) {
        if rate == self.rate {
            return;
        }
        let pos = self.position();
        self.rate = rate;
        self.sh.rate.store(rate, Ordering::Relaxed);
        self.sh.dsp.lock().set_rate(rate);
        self.sh.scope.lock().rate = rate;
        if let Some(c) = &mut self.cur {
            c.rs = Resample::new(c.src.info.rate, rate, self.speed);
            c.pend.clear();
        }
        self.next = None;
        // Utwór od tego samego miejsca z nowym przepróbkowaniem.
        if reseek && self.cur.is_some() {
            self.seek(pos);
        }
    }

    /// Częstotliwość, w której najlepiej zagra bieżący utwór.
    fn wanted_rate(&self) -> u32 {
        self.cur.as_ref().map(|c| c.src.info.rate).filter(|r| *r > 0).unwrap_or(self.rate)
    }

    fn open_exclusive(&mut self, want: u32, reseek: bool) -> bool {
        self.excl = None;
        match super::exclusive::Exclusive::open(self.device.clone(), want, self.sh.clone()) {
            Ok(x) => {
                self.stream = None;
                self.sh.out_bits.store(x.bits, Ordering::Relaxed);
                self.sh.exclusive.store(true, Ordering::Relaxed);
                self.sh.failed.store(false, Ordering::Relaxed);
                let rate = x.rate;
                self.excl = Some(x);
                self.set_rate(rate, reseek);
                true
            }
            Err(e) => {
                self.want_excl = false;
                let _ = self.etx.send(Event::Device(crate::tr!("Exclusive mode unavailable ({}) – playing through the Windows mixer", e)));
                false
            }
        }
    }

    fn open_stream(&mut self) {
        self.stream = None;
        self.excl = None;
        self.sh.exclusive.store(false, Ordering::Relaxed);
        if self.want_excl && self.open_exclusive(self.wanted_rate(), true) {
            return;
        }
        self.sh.out_bits.store(0, Ordering::Relaxed);
        let host = cpal::default_host();
        let dev = self
            .device
            .as_ref()
            .and_then(|n| host.output_devices().ok()?.find(|d| d.name().ok().as_deref() == Some(n)))
            .or_else(|| host.default_output_device());
        let Some(dev) = dev else {
            let _ = self.etx.send(Event::Device(crate::tr!("No audio device").into()));
            return;
        };
        let Ok(cfg) = dev.default_output_config() else { return };
        let rate = cfg.sample_rate().0;
        let ch = cfg.channels() as usize;
        let sc: cpal::StreamConfig = cfg.config();
        let res = match cfg.sample_format() {
            cpal::SampleFormat::F32 => build::<f32>(&dev, &sc, ch, rate, None, self.sh.clone()),
            cpal::SampleFormat::I16 => build::<i16>(&dev, &sc, ch, rate, Some(16), self.sh.clone()),
            cpal::SampleFormat::I32 => build::<i32>(&dev, &sc, ch, rate, None, self.sh.clone()),
            cpal::SampleFormat::U16 => build::<u16>(&dev, &sc, ch, rate, Some(16), self.sh.clone()),
            _ => build::<f32>(&dev, &sc, ch, rate, None, self.sh.clone()),
        };
        match res {
            Ok(s) => {
                let _ = s.play();
                self.stream = Some(s);
                self.session = super::exclusive::session_volume(self.device.as_deref());
                self.session_set = -1.0;
                self.sh.session.store(self.session.is_some(), Ordering::Relaxed);
                self.sh.failed.store(false, Ordering::Relaxed);
                self.set_rate(rate, true);
            }
            Err(e) => {
                let _ = self.etx.send(Event::Device(crate::tr!("Could not open the audio output: {}", e)));
            }
        }
        if self.want_loopback {
            self.open_loopback();
        }
    }

    fn open_loopback(&mut self) {
        self.loopback = None;
        let host = cpal::default_host();
        let Some(dev) = host.default_output_device() else { return };
        let Ok(cfg) = dev.default_output_config() else { return };
        let ch = cfg.channels() as usize;
        let sh = self.sh.clone();
        let mut st = Vec::new();
        // Wejście na urządzeniu wyjściowym = przechwytywanie pętli zwrotnej WASAPI.
        let s = dev.build_input_stream(
            &cfg.config(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                st.clear();
                for f in data.chunks_exact(ch.max(1)) {
                    let l = f[0];
                    let r = if ch > 1 { f[1] } else { l };
                    st.push(l);
                    st.push(r);
                }
                if let Some(mut s) = sh.scope.try_lock() {
                    s.push(&st);
                }
            },
            |_| {},
            None,
        );
        if let Ok(s) = s {
            let _ = s.play();
            self.loopback = Some(s);
        }
    }

    fn position(&self) -> f64 {
        let r = self.sh.ring.lock();
        match r.marks.back().or(r.cur.as_ref()) {
            Some(m) if self.cur.as_ref().is_some_and(|c| c.id == m.id) => {
                m.base + (r.played.max(m.frame) - m.frame) as f64 / self.rate as f64 * m.speed
            }
            _ => 0.0,
        }
    }

    fn run(mut self) {
        loop {
            let busy = self.cur.is_some() || self.switch.is_some();
            let cmd = if busy { self.rx.recv_timeout(Duration::from_millis(8)) } else { self.rx.recv_timeout(Duration::from_millis(200)) };
            match cmd {
                Ok(c) => {
                    self.handle(c);
                    while let Ok(c) = self.rx.try_recv() {
                        self.handle(c);
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return,
                Err(_) => {}
            }
            if self.sh.failed.swap(false, Ordering::Relaxed) || (self.stream.is_none() && self.excl.is_none()) {
                // Odłączone słuchawki / zmiana urządzenia domyślnego – otwieramy wyjście ponownie.
                std::thread::sleep(Duration::from_millis(300));
                self.open_stream();
            }
            self.sync_session();
            self.pump();
        }
    }

    /// Głośność aplikacji ↔ suwak OpenMP w mikserze Windows (w obie strony).
    fn sync_session(&mut self) {
        if self.stream.is_none() || self.excl.is_some() {
            if self.session.take().is_some() {
                self.sh.session.store(false, Ordering::Relaxed);
            }
            return;
        }
        let Some(v) = &self.session else { return };
        let want = f32::from_bits(self.sh.vol.load(Ordering::Relaxed));
        let now = unsafe { v.GetMasterVolume() }.unwrap_or(self.session_set);
        if self.session_set >= 0.0 && (now - self.session_set).abs() > 0.002 {
            // Ktoś przesunął suwak OpenMP w mikserze Windows.
            self.session_set = now;
            self.sh.vol.store(now.to_bits(), Ordering::Relaxed);
            let _ = self.etx.send(Event::Volume(now.max(0.0).powf(2.0 / 3.0)));
        } else if (want - self.session_set).abs() > 0.0001 {
            if unsafe { v.SetMasterVolume(want, std::ptr::null()) }.is_ok() {
                self.session_set = want;
            }
        }
    }

    fn handle(&mut self, c: Cmd) {
        match c {
            Cmd::Load(it, start) => {
                self.switch = None;
                self.fade = None;
                self.next = None;
                match Track::open(&it, self.rate, self.speed) {
                    Ok(mut t) => {
                        let base = if start > 0.5 { t.src.seek(start) } else { 0.0 };
                        self.sh.ring.lock().cut(self.rate);
                        // Tryb wyłączny: karta przechodzi na częstotliwość pliku (bez przepróbkowania).
                        let want = t.src.info.rate;
                        if self.excl.is_some() && want > 0 && want != self.rate {
                            self.cur = None;
                            if !self.open_exclusive(want, false) {
                                self.open_stream();
                            }
                            t.rs = Resample::new(want, self.rate, self.speed);
                        }
                        let mut r = self.sh.ring.lock();
                        let m = t.mark(r.written, base, self.speed);
                        r.marks.push_back(m);
                        drop(r);
                        t.fade_in = (self.rate as f64 * DECLICK_SECS) as usize;
                        self.cur = Some(t);
                    }
                    Err(msg) => {
                        self.cur = None;
                        let mut r = self.sh.ring.lock();
                        r.cut(self.rate);
                        r.marks.clear();
                        r.cur = None;
                        drop(r);
                        let _ = self.etx.send(Event::Error { id: it.id, path: it.path, msg });
                    }
                }
            }
            Cmd::Next(it) => {
                if self.next_item.as_ref().map(|i| (i.id, &i.path)) != it.as_ref().map(|i| (i.id, &i.path)) {
                    self.next = None;
                    if self.fade.is_some() {
                        // Przenikanie w toku z innym utworem – zostawiamy, aplikacja i tak przejdzie dalej.
                    }
                }
                self.next_item = it;
            }
            Cmd::Seek(t) => self.seek(t),
            Cmd::Stop => {
                self.switch = None;
                self.cur = None;
                self.next = None;
                self.fade = None;
                let mut r = self.sh.ring.lock();
                r.cut(self.rate);
                r.marks.clear();
                r.cur = None;
            }
            Cmd::Speed(s) => {
                let s = s.clamp(0.25, 4.0);
                if (s - self.speed).abs() > 1e-6 {
                    let pos = self.position();
                    self.speed = s;
                    if let Some(c) = &mut self.cur {
                        c.rs.set_speed(s);
                        let mut r = self.sh.ring.lock();
                        let m = c.mark(r.written, pos + r.buf.len() as f64 / 2.0 / self.rate as f64, s);
                        if self.fade.is_none() {
                            r.marks.push_back(m);
                        }
                    }
                    if let Some(n) = &mut self.next {
                        n.rs.set_speed(s);
                    }
                }
            }
            Cmd::Crossfade(s) => self.crossfade = s.clamp(0.0, 20.0),
            Cmd::Device(d) => {
                self.device = d;
                self.open_stream();
            }
            Cmd::Gain(id, g) => {
                for t in [&mut self.cur, &mut self.next, &mut self.switch].into_iter().flatten() {
                    if t.id == id {
                        t.gain = g;
                    }
                }
            }
            Cmd::Exclusive(on) => {
                if on != self.want_excl {
                    self.want_excl = on;
                    self.open_stream();
                }
            }
            Cmd::Loopback(on) => {
                self.want_loopback = on;
                if on {
                    self.open_loopback();
                } else {
                    self.loopback = None;
                }
            }
        }
    }

    fn seek(&mut self, t: f64) {
        let Some(c) = &mut self.cur else { return };
        if self.fade.is_some() {
            // Przewijanie w trakcie przenikania – zostaje już tylko nowy utwór.
            self.fade = None;
            if let Some(n) = self.next.take() {
                *c = n;
                self.next_item = None;
            }
        }
        self.next = None;
        let dur = c.src.info.duration;
        let t = if dur > 0.0 { t.clamp(0.0, (dur - 0.05).max(0.0)) } else { t.max(0.0) };
        let base = c.src.seek(t);
        c.rs.reset();
        c.pend.clear();
        c.eof = false;
        c.fade_in = (self.rate as f64 * DECLICK_SECS) as usize;
        let mut r = self.sh.ring.lock();
        r.cut(self.rate);
        let m = c.mark(r.written, base, self.speed);
        r.marks.push_back(m);
    }

    fn open_next(&mut self) {
        if self.next.is_some() {
            return;
        }
        let Some(it) = self.next_item.clone() else { return };
        match Track::open(&it, self.rate, self.speed) {
            Ok(t) => self.next = Some(t),
            Err(msg) => {
                self.next_item = None;
                let _ = self.etx.send(Event::Error { id: it.id, path: it.path, msg });
            }
        }
    }

    /// Dopełnia bufor wyjścia do AHEAD_SECS.
    /// Czy utwór wymaga przełączenia karty na inną częstotliwość (tylko tryb wyłączny).
    fn needs_switch(&self, t: &Track) -> bool {
        self.excl.is_some() && t.src.info.rate > 0 && t.src.info.rate != self.rate
    }

    /// Bufor wybrzmiał – karta na częstotliwość czekającego utworu i od razu dalej.
    fn finish_switch(&mut self) {
        let Some(mut nx) = self.switch.take() else { return };
        let want = nx.src.info.rate;
        if !self.open_exclusive(want, false) {
            self.open_stream();
        }
        nx.rs = Resample::new(want, self.rate, self.speed);
        nx.pend.clear();
        nx.fade_in = 0;
        let mut r = self.sh.ring.lock();
        let m = nx.mark(r.written, 0.0, self.speed);
        r.marks.push_back(m);
        drop(r);
        self.cur = Some(nx);
    }

    fn pump(&mut self) {
        if self.switch.is_some() && self.sh.ring.lock().buf.is_empty() {
            self.finish_switch();
        }
        let target = (self.rate as f64 * AHEAD_SECS) as usize;
        for _ in 0..64 {
            if self.cur.is_none() {
                return;
            }
            let have = self.sh.ring.lock().buf.len() / 2;
            if have >= target {
                return;
            }
            self.produce(1024);
        }
    }

    fn produce(&mut self, n: usize) {
        let rate = self.rate;
        self.chunk.clear();
        let written = self.sh.ring.lock().written;
        let mut new_marks: Vec<Mark> = Vec::new();
        let mut ended = false;
        {
            let Some(c) = &mut self.cur else { return };
            c.fill(n);
        }
        // Zapowiedź następnego: otwieramy go kilka sekund wcześniej, żeby przejście było płynne.
        let remaining = self.cur.as_ref().map(|c| c.remaining(rate)).unwrap_or(0.0);
        if self.fade.is_none() && self.next_item.is_some() && remaining < self.crossfade + 4.0 {
            self.open_next();
        }
        if self.fade.is_none() && self.crossfade > 0.0 && self.next.as_ref().is_some_and(|n| !self.needs_switch(n)) && remaining <= self.crossfade && remaining.is_finite() {
            let len = ((remaining.min(self.crossfade)) * rate as f64) as usize;
            if len > rate as usize / 4 {
                self.fade = Some((0, len));
                let n = self.next.as_ref().unwrap();
                new_marks.push(n.mark(written, 0.0, self.speed));
            }
        }
        if let Some((pos, len)) = self.fade {
            let nx = self.next.as_mut().unwrap();
            nx.fill(n);
            let c = self.cur.as_mut().unwrap();
            for i in 0..n {
                let t = ((pos + i) as f32 / len as f32).min(1.0);
                let (ga, gb) = ((t * std::f32::consts::FRAC_PI_2).cos(), (t * std::f32::consts::FRAC_PI_2).sin());
                let (al, ar) = c.take().unwrap_or((0.0, 0.0));
                let (bl, br) = nx.take().unwrap_or((0.0, 0.0));
                self.chunk.push(al * ga + bl * gb);
                self.chunk.push(ar * ga + br * gb);
            }
            let pos = pos + n;
            let done = pos >= len || (c.eof && c.pend.is_empty());
            self.fade = Some((pos, len));
            if done {
                self.fade = None;
                self.cur = self.next.take();
                self.next_item = None;
            }
        } else {
            let mut i = 0;
            while i < n {
                let Some(c) = &mut self.cur else { break };
                match c.take() {
                    Some((l, r)) => {
                        let g = if c.fade_in > 0 { 1.0 - c.fade_in as f32 / (rate as f64 * DECLICK_SECS) as f32 } else { 1.0 };
                        self.chunk.push(l * g);
                        self.chunk.push(r * g);
                        i += 1;
                    }
                    None if c.eof => {
                        // Bez przerwy: następny utwór zaczyna się w tej samej porcji.
                        self.open_next();
                        match self.next.take() {
                            Some(nx) if self.excl.is_some() && nx.src.info.rate > 0 && nx.src.info.rate != rate => {
                                // Tryb wyłączny: karta przejdzie na częstotliwość następnego utworu,
                                // gdy wybrzmi to, co już jest w buforze.
                                self.switch = Some(nx);
                                self.cur = None;
                                self.next_item = None;
                                break;
                            }
                            Some(mut nx) => {
                                nx.fade_in = 0;
                                new_marks.push(nx.mark(written + i as u64, 0.0, self.speed));
                                self.cur = Some(nx);
                                self.next_item = None;
                            }
                            None => {
                                self.cur = None;
                                ended = true;
                            }
                        }
                    }
                    None => {
                        c.fill(n - i);
                        if c.pend.is_empty() && !c.eof {
                            break;
                        }
                    }
                }
            }
        }
        let mut r = self.sh.ring.lock();
        // Bufor mógł zostać obcięty w międzyczasie (nie powinien – ten sam wątek), na wszelki wypadek.
        let base = r.written;
        for mut m in new_marks {
            m.frame = m.frame - written + base;
            r.marks.push_back(m);
        }
        r.buf.extend(self.chunk.iter().copied());
        r.written += (self.chunk.len() / 2) as u64;
        if ended {
            r.end_frame = Some(r.written);
        }
    }
}

/// Szum TPDF o amplitudzie jednego najmłodszego bitu – kwantyzacja bez zniekształceń w cichych partiach.
#[derive(Default)]
pub(super) struct Dither(u32);

impl Dither {
    fn uniform(&mut self) -> f32 {
        let mut x = if self.0 == 0 { 0x9e37_79b9 } else { self.0 };
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Szum w zakresie ±1 LSB.
    pub(super) fn noise(&mut self) -> f32 {
        self.uniform() - self.uniform()
    }

    /// Próbka −1..1 → liczba całkowita o `bits` bitach (dither do 24 bitów włącznie).
    pub(super) fn quantize(&mut self, x: f32, bits: u32) -> i32 {
        let scale = ((1i64 << (bits - 1)) - 1) as f64;
        let mut v = x.clamp(-1.0, 1.0) as f64 * scale;
        if bits <= 24 {
            v += self.noise() as f64;
        }
        v.round().clamp(-scale - 1.0, scale) as i32
    }
}

/// Kolejna porcja stereo dla karty: bufor → korektor → wizualizacja → głośność z płynną pauzą.
pub(super) fn render(sh: &Shared, tmp: &mut Vec<f32>, frames: usize, rate: u32, fade: &mut f32) {
    let paused = sh.paused.load(Ordering::Relaxed);
    let soft = sh.soft.load(Ordering::Relaxed);
    let target = if paused { 0.0 } else { 1.0 };
    if !soft {
        *fade = target;
    }
    tmp.clear();
    tmp.resize(frames * 2, 0.0);
    if paused && *fade <= 0.0 {
        return;
    }
    {
        let mut r = sh.ring.lock();
        let n = (frames * 2).min(r.buf.len());
        for (i, v) in r.buf.drain(..n).enumerate() {
            tmp[i] = v;
        }
        r.played += (n / 2) as u64;
    }
    sh.dsp.lock().process(tmp);
    if let Some(mut s) = sh.scope.try_lock() {
        s.push(tmp);
    }
    // Przez mikser Windows głośność nakłada sesja (z głośnością główną) – tu tylko płynna pauza.
    let vol = if sh.session.load(Ordering::Relaxed) { 1.0 } else { f32::from_bits(sh.vol.load(Ordering::Relaxed)) };
    let step = 1.0 / (0.12 * rate as f32);
    for f in tmp.chunks_exact_mut(2) {
        if *fade < target {
            *fade = (*fade + step).min(1.0);
        } else if *fade > target {
            *fade = (*fade - step).max(0.0);
        }
        let g = vol * *fade * *fade;
        f[0] *= g;
        f[1] *= g;
    }
}

fn build<T>(dev: &cpal::Device, cfg: &cpal::StreamConfig, ch: usize, rate: u32, dither: Option<u32>, sh: Arc<Shared>) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let mut tmp: Vec<f32> = Vec::with_capacity(16384);
    let mut fade = 0.0f32;
    let mut d = Dither::default();
    let lsb = dither.map(|b| 1.0 / (1u32 << (b - 1)) as f32);
    let sh2 = sh.clone();
    dev.build_output_stream(
        cfg,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let frames = data.len() / ch.max(1);
            render(&sh, &mut tmp, frames, rate, &mut fade);
            if let Some(lsb) = lsb {
                for s in tmp.iter_mut() {
                    *s += d.noise() * lsb;
                }
            }
            for (f, out) in data.chunks_exact_mut(ch.max(1)).enumerate() {
                let (l, r) = (tmp[f * 2], tmp[f * 2 + 1]);
                match ch {
                    1 => out[0] = T::from_sample((l + r) * 0.5),
                    _ => {
                        out[0] = T::from_sample(l);
                        out[1] = T::from_sample(r);
                        for o in out.iter_mut().skip(2) {
                            *o = T::from_sample(0.0);
                        }
                    }
                }
            }
        },
        move |_| sh2.failed.store(true, Ordering::Relaxed),
        None,
    )
    .map_err(|e| e.to_string())
}

/// Czy silnik muzyki zagra ten plik (inaczej odtwarza mpv).
pub fn can_play(path: &Path) -> bool {
    super::decoder::supported(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dither_w_granicach_jednego_bitu() {
        let mut d = Dither::default();
        let scale = ((1i64 << 15) - 1) as f64;
        let mut sum = 0.0;
        for i in 0..20000 {
            let x = (i as f32 / 20000.0) * 1.6 - 0.8;
            let q = d.quantize(x, 16) as f64;
            let exact = x as f64 * scale;
            assert!((q - exact).abs() <= 2.0, "{q} vs {exact}");
            sum += q - exact;
        }
        // Szum TPDF jest symetryczny – brak przesunięcia stałego.
        assert!((sum / 20000.0).abs() < 0.05);
    }

    #[test]
    fn kwantyzacja_bez_przesterowania() {
        let mut d = Dither::default();
        // Dither może zejść o jeden bit poniżej szczytu, nigdy ponad zakres.
        assert!((32766..=32767).contains(&d.quantize(2.0, 16)));
        assert!((-32768..=-32766).contains(&d.quantize(-2.0, 16)));
        assert_eq!(d.quantize(1.0, 32), i32::MAX);
        assert_eq!(d.quantize(0.0, 32), 0);
    }

    #[test]
    fn obciecie_bufora_z_wygaszeniem() {
        let mut r = Ring { buf: std::iter::repeat(1.0).take(48000).collect(), played: 100, written: 24100, marks: VecDeque::new(), cur: None, end_frame: Some(5) };
        r.cut(48000);
        let n = (48000.0 * DECLICK_SECS) as usize;
        assert_eq!(r.buf.len(), n * 2);
        assert_eq!(r.written, 100 + n as u64);
        assert!(r.buf[0] > 0.9 && r.buf[r.buf.len() - 1] < 0.05);
        assert!(r.end_frame.is_none());
    }
}
