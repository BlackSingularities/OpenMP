# OpenMP

Odtwarzacz muzyki i wideo dla Windows 10 i 11. Natywny program w Rust ze Slint: szybki start, jeden lekki plik i własny silnik dźwięku.

## Muzyka

- **Teraz odtwarzane** – duża okładka z poświatą, szeryfowy tytuł, wykonawca i album, etykiety formatu (format, częstotliwość, bitrate, kanały). Akcent całego interfejsu płynie z okładki bieżącego utworu.
- **Wizualizacja na okładce** – słupki ze szczytami, widmo, oscyloskop albo wskaźniki VU liczone wprost z próbek, nałożone na dolną część okładki (przełącznik na okładce, można też wyłączyć). Widoczna w każdym widoku.
- **Taśma utworu** – pasek przewijania z falą całego utworu w czterech stylach (fala, słupki, od dołu, kontur), z podglądem czasu pod kursorem i znacznikami pętli A-B. Ten sam pasek jest w każdym widoku.
- **Silnik dźwięku** – przejścia bez przerw, płynne przenikanie, korektor 18-pasmowy z presetami i własnymi ustawieniami, ReplayGain (utwór lub album) z ogranicznikiem, szybkość i wysokość dźwięku, wybór urządzenia wyjściowego, łagodna pauza.
- **Teksty** – pliki `.lrc` obok utworu, teksty z tagów i [LRCLIB](https://lrclib.net) dla brakujących (do serwera trafiają tylko wykonawca, tytuł, album i czas trwania). Wersje z sygnaturami czasu przewijają się razem z utworem, a pobrany tekst można zapisać obok pliku.
- **Listy** – wiele kart z listami, grupowanie według albumów, kolejka, historia, losowanie i powtarzanie, „zatrzymaj po bieżącym”, wyszukiwanie, przeciąganie plików i folderów, import M3U/M3U8, PLS i CUE, eksport do M3U.
- **Edytor tagów** – także dla wielu plików naraz, z okładką.
- **Radio internetowe** – wyszukiwarka stacji, ulubione stacje i tytuły z nadawanego strumienia.

## Wideo

- Obraz na całe okno i mały panel sterowania, który sam się chowa.
- Dekodowanie sprzętowe, napisy (ASS, SRT, PGS), ścieżki dźwiękowe, rozdziały na pasku, wznawianie od miejsca przerwania, zrzuty klatek, usuwanie przeplotu, dopasowanie okna do filmu, pełny ekran.
- Silnik wideo to libmpv, więc obsługiwany jest praktycznie każdy format.

## Biblioteka

- Osobne biblioteki muzyki i wideo z własnymi listami folderów.
- Dyski sieciowe: ścieżki UNC (`\serwer\udział`) i dyski zmapowane. Niedostępny serwer NAS nie usuwa wpisów z biblioteki.
- Muzyka: wykonawcy, albumy (z okładkami), gatunki, lata, foldery, ulubione, najczęściej odtwarzane, ostatnio dodane i historia, ze statystykami odtworzeń.
- Wideo: wszystkie filmy, foldery, „Kontynuuj oglądanie” i ostatnio dodane.

## Widoki

- **Duży**: panel nawigacji z folderami biblioteki, okładka z opisem nad listą lub biblioteką, panel boczny (tekst, informacje, kolejka) i dolny pasek z taśmą utworu, przyciskami odtwarzania, wyjściem dźwięku i głośnością.
- **Pomniejszony** (Ctrl+K): okładka z wizualizacją i tytuł po lewej, ten sam dolny pasek.
- **Miniodtwarzacz** (Ctrl+M): okno na wierzchu z tytułem i paskiem postępu, na którym są przyciski odtwarzania; okładka z wizualizacją pojawia się nad paskiem po najechaniu.

## Windows

Klawisze multimedialne i systemowa nakładka odtwarzania, skojarzenia plików i „Dodaj do listy OpenMP” w menu kontekstowym, jedna instancja, wyłącznik czasowy, blokada usypiania podczas filmu. Program sam pobiera i instaluje kolejne wersje z redzikowski.com.

## Instalacja

Pobierz `OpenMP-Setup-x.y.z.exe` ze strony [redzikowski.com](https://redzikowski.com) i uruchom go. Instalacja dotyczy bieżącego użytkownika i nie wymaga uprawnień administratora. Do wyboru jest też wersja przenośna `OpenMP-x.y.z-portable.zip`.

## Budowanie

```
cargo build --release -p openmp
```

Do odtwarzania wideo obok `OpenMP.exe` musi leżeć `libmpv-2.dll` (np. z [mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake/releases), paczka `mpv-dev-x86_64`). Wydanie powstaje wyłącznie po wypchnięciu tagu `vX.Y.Z`. CI buduje wtedy program, instalator (Inno Setup) i wersję przenośną, liczy sumy SHA-256 i publikuje wydanie z numerem wersji z tagu.

## Licencja

MIT. Wideo: [libmpv](https://mpv.io) (LGPL).

---

**Pomysł i wykonanie: Adam Rędzikowski · [redzikowski.com](https://redzikowski.com)**

<a href="https://buycoffee.to/adamredzikowski" target="_blank"><img src="https://buycoffee.to/static/img/share/share-button-dark--pl.png" style="width: 351px; height: 92px" width="351" height="92" alt="Postaw kawę dla adamredzikowski na buycoffee.to"></a>
