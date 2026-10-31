//! OpenMP – rdzeń: silnik muzyki, warstwa mpv, metadane, biblioteka, playlisty, ustawienia, aktualizacje.

pub mod audio;
pub mod coverart;
pub mod instance;
pub mod library;
pub mod listenbrainz;
pub mod lrclib;
pub mod meta;
pub mod mpv;
pub mod paths;
pub mod playlist;
pub mod radio;
pub mod settings;
pub mod update;

/// Wersja programu (CI podaje numer z tagu przy kompilacji).
pub fn version() -> &'static str {
    option_env!("OPENMP_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

/// Autor i wsparcie – jedno źródło dla okna „O programie”, stopki ustawień i instalatora.
pub const AUTHOR_LINE: &str = "Pomysł i wykonanie: Adam Rędzikowski · redzikowski.com";
pub const AUTHOR_URL: &str = "https://redzikowski.com";
pub const SUPPORT_URL: &str = "https://buycoffee.to/adamredzikowski";
