# Zmiany

## 1.2.4

- Panel nawigacji bez pozycji „Foldery” i „Foldery wideo” – foldery są w sekcji FOLDERY.
- Folder z sekcji FOLDERY otwiera się jako drzewo podfolderów zamiast płaskiej listy wszystkich plików.

## 1.2.3

- Globalne skróty klawiszowe, działające także przy oknie w tle: Ctrl+Alt+Spacja (odtwórz/pauza), Ctrl+Alt+←/→ (utwór), Ctrl+Alt+↑/↓ (głośność). Można je wyłączyć w ustawieniach.

## 1.2.2

- ReplayGain bez tagów: pliki bez wpisów ReplayGain są mierzone w tle (EBU R128) i wyrównywane tak samo jak otagowane.
- Tryb wyłączny: utwór o innej częstotliwości przełącza kartę zamiast być przepróbkowany.
- Obsługa klawiaturą: Tab między przyciskami, przełącznikami i suwakami, Spacja/Enter, strzałki, Home/End.
- Testy silnika dźwięku (dither, przepróbkowanie, miks do stereo, wygaszanie bufora).

## 1.2.1

- Przyciski na miniaturze w pasku zadań: poprzedni, odtwórz/pauza, następny.
- Brakujące okładki albumów pobierane z MusicBrainz i Cover Art Archive (można wyłączyć w ustawieniach).
- Inteligentne listy: zmiana nazwy i filtrów z menu w panelu.

## 1.2.0

- Mocniejszy, pełniejszy dźwięk: ReplayGain domyślnie wyłączony (ściszał głośne nagrania o 6–10 dB), łagodniejsza krzywa głośności, ogranicznik tylko przy realnym przesterowaniu.
- Tryb wyłączny WASAPI (Ustawienia → Dźwięk): karta gra w częstotliwości pliku, z pominięciem miksera Windows.
- Dither TPDF przy wyjściu 16- i 24-bitowym, wybór jakości przepróbkowania, plakietka drogi do karty („Wyłączny”, „→ 48 kHz”).
- Głośność wideo i radia (mpv) zgodna z głośnością muzyki.
- Foldery w bibliotece jako drzewo: podfoldery jako kafelki, pliki obok nich osobno.
- Obserwowanie folderów biblioteki – nowe i usunięte pliki pojawiają się same.
- Oceny 1–5 (Ctrl+1…5, Ctrl+0 usuwa) i widok „Ocenione”.
- Filtry w wyszukiwaniu (ocena>=4, rok:1990-1999, gatunek:rock, format:flac, dodane<30…) i zapisywane inteligentne listy.
- Scrobbling do ListenBrainz (token w ustawieniach).
- Obsługa czytników ekranu (przyciski, przełączniki, suwaki).
- Szybszy widok biblioteki przy dużych kolekcjach.

## 1.1.3

- Program nie zawiesza się już po zamknięciu okna wyboru folderu albo pliku bez wyboru. Okna wyboru (pliki, foldery, biblioteka, okładka, napisy, zapis listy) otwierają się w osobnym wątku, nad oknem programu albo dialogu, który je wywołał.

## 1.1.2

- Ustawienia znów się otwierają: menu było zbudowane na oknie wyskakującym, które gubiło kliknięcia w pozycje.
- Menu (główne, kontekstowe, wyłącznik czasowy, wybór urządzenia) jako osobne okna – mogą wystawać poza okno programu, w wyglądzie i kolorze akcentu OpenMP.
- Okna dialogowe (ustawienia, korektor, edytor tagów, aktualizacja, adres, zmiana nazwy, o programie) jako osobne okna, wyśrodkowane nad programem, przeciągane za pasek tytułu.
- Wybór urządzenia wyjściowego jako sama ikona za suwakiem głośności, lista po kliknięciu.
- Miniodtwarzacz: okładka bez wizualizacji (została ta w tle) i płaski dolny pasek.

## 1.1.1

- Naprawione zawieszanie przy zamykaniu: proces potrafił utknąć z widocznym, martwym oknem, w którym nie działał żaden przycisk ani ustawienia, a kolejne uruchomienia trafiały do niego i nic się nie otwierało. Program kończy się teraz od razu po zapisaniu stanu, a nowe uruchomienie oddaje pliki tylko działającemu oknu, które to potwierdzi.
- Korektor dopasowany do gatunku (jak w OpenAMP): przełącznik „Dopasuj do gatunku” w oknie korektora dobiera preset z tagu gatunku przy każdej zmianie utworu.
- Okna dialogowe (ustawienia, korektor, edytor tagów) zawsze w całości: w widoku pomniejszonym i w miniodtwarzaczu okno programu rośnie na czas dialogu i potem wraca do swojego rozmiaru.
- Dziennik programu w `%LOCALAPPDATA%\OpenMP\openmp.log` (awarie z miejscem wystąpienia, start i zamknięcie).

## 1.1.0

- Nowy wygląd całego programu: głęboka czerń i jeden akcent, który płynie z okładki bieżącego utworu przez wszystkie widoki (przyciski, zaznaczenia, taśma, poświata).
- Widok duży: panel nawigacji, duża okładka z poświatą i opisem „Teraz odtwarzane” (szeryfowy tytuł, wykonawca, etykiety formatu), pod spodem listy i biblioteka, po prawej tekst, informacje i kolejka.
- Wizualizacja na okładce (jak w Poweramp): słupki, widmo, oscyloskop albo wskaźniki VU; przełącznik na okładce, można ją też wyłączyć.
- Dolny pasek na całą szerokość: duża taśma utworu z czasami, przyciski odtwarzania na środku, wybór urządzenia wyjściowego i głośność po lewej, przełączniki po prawej.
- Widok pomniejszony: okładka z wizualizacją i tytuł po lewej, ten sam dolny pasek.
- Miniodtwarzacz: przyciski odtwarzania na pasku postępu, tytuł zawsze widoczny, okładka z wizualizacją nad paskiem po najechaniu, delikatny analizator za tytułem.
- Panel nawigacji: foldery biblioteki (także sieciowe) jako skróty do całej zawartości, listy odtwarzania pod jedną pozycją; poprawione ikony.
- Taśma utworu domyślnie w stylu słupków.
- Poprawione menu główne – nie jest już ucinane przy lewej krawędzi.

## 1.0.0

- Pierwsze wydanie: tryb muzyki z rozbudowanym wyświetlaczem i wizualizacjami, tryb wideo z chowającym się panelem, miniodtwarzacz.
- Własny silnik dźwięku: przejścia bez przerw, przenikanie, korektor 18-pasmowy, ReplayGain, szybkość i wysokość.
- Biblioteka muzyki i wideo, także na dyskach sieciowych (UNC i zmapowanych).
- Taśma utworu na pasku przewijania w czterech stylach, ten sam pasek w miniodtwarzaczu.
- Analizator widoczny także w widoku kompaktowym i w miniodtwarzaczu.
- Teksty piosenek z plików `.lrc`, tagów i LRCLIB, z zapisem pobranego tekstu obok utworu.
- Radio internetowe, edytor tagów, wyłącznik czasowy, pętla A-B, klawisze multimedialne i nakładka systemowa.
- Instalator dla bieżącego użytkownika, wersja przenośna i samoaktualizacja przez redzikowski.com.
