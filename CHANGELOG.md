# Zmiany

## 1.5.2

- **Kafelki i wiersze bez ikonek** serduszka i odtwarzania. Wszędzie (biblioteka, foldery, historia, listy) oznaczony jest tylko grający plik i pliki, które są już na karcie „Kolejka” („W kolejce”).
- **Zaznaczenie z opisem:** film – polski tytuł, rok, gatunki, ocena, opis, reżyseria i obsada z TMDB; utwór – komentarz z tagów. Obok, gdy jest dostępny, tekst piosenki.
- Szczegóły zaznaczenia w dwóch kolumnach; przy braku miejsca znikają całe wiersze od dołu zamiast uciętych w pół.
- **Klik w wiersz tekstu obok opisu** przewija utwór albo film do tej chwili; w zaznaczeniu utwór zaczyna grać od tej linii.
- Zamknięcie karty „Kolejka” zaczyna ją od nowa – pustą albo z grającym utworem.

## 1.5.1

- Granie z biblioteki dopisuje utwory na koniec karty „Kolejka” zamiast ją zastępować – to, co już było w kolejce, zostaje.

## 1.5.0

Porządek w trybach okna – są trzy: okno, pełny ekran i miniodtwarzacz.

- **Okno i pełny ekran mają ten sam układ:** okładka (albo obraz filmu) z opisem obok, lista, panel boczny i dolny pasek. Pełny ekran okna – F11 albo przycisk na dolnym pasku; Esc wraca do okna.
- **Film gra w oknie obok opisu**, jak muzyka. Kliknięcie obrazu, F albo przycisk otwiera pełny ekran filmu z samym obrazem; Esc wraca do okna, a film gra dalej. Osobny „szeroki widok filmu” zniknął.
- **Miniodtwarzacz** (Ctrl+M) bez zmian; „Widok kompaktowy” usunięty.
- **Panel nawigacji** można włączyć zawsze poza miniodtwarzaczem – przyciskiem albo Ctrl+B, także na pełnym ekranie.
- **Karta „Kolejka” – bieżące odtwarzanie:** granie z biblioteki, otwarte pliki i pozycje „Dodaj do kolejki”/„Odtwórz jako następne” z innych list trafiają na jedną kartę zamiast tworzyć nowe listy. Dawna karta „Biblioteka” staje się „Kolejką”. Nowe listy powstają tylko przy imporcie albo ręcznie.
- Ustawienie „Otwieraj filmy na pełnym ekranie” (domyślnie wyłączone – film zaczyna grać w oknie).

## 1.3.0

Wideo przestaje być osobnym trybem – jest częścią programu.

- **Film nie zamyka interfejsu.** Esc albo strzałka „Biblioteka” wraca do biblioteki, a film gra dalej w miejscu okładki (z poświatą); kliknięcie obrazu wraca do widoku filmu. Do wyboru też: tylko dźwięk albo pauza.
- **Dwa widoki filmu:** kinowy (obraz na całe okno, chowające się sterowanie) i szeroki (obraz na scenie z panelem listy i ustawień, sterowanie jak w oknie muzyki). Przełączanie klawiszem W. Na pełnym ekranie zawsze autoukrywanie sterowania i kursora.
- **Obraz w obrazie** (P): miniodtwarzacz w proporcjach filmu w rogu ekranu, zawsze na wierzchu, z przyciskami po najechaniu.
- **Poświata z obrazu (Ambilight):** miękkie światło w barwach filmu wokół obrazu i w czarnych pasach; kolor przewodni programu płynnie podąża za filmem.
- **Podgląd klatek** nad paskiem przewijania, z nazwą rozdziału; zbuforowana część strumienia na pasku.
- Silnik obrazu: mpv rysuje dokładnie w rozmiarze miejsca na ekranie (mniej pracy karty graficznej), cztery poziomy jakości skalowania, wybór dekodowania sprzętowego (D3D11, NVDEC, DXVA2), płynny ruch (interpolacja), usuwanie pasm, mapowanie tonów HDR, plakietki 4K/HDR10/kodek.
- Obraz: powiększenie (Ctrl+kółko, +/−), przesuwanie, obrót, proporcje „Wypełnij ekran”, odcień; „Obraz jak w pliku” (0).
- Dźwięk filmów: wzmocnienie do 200%, tryb nocny (wyrównana dynamika), miks do stereo, szybkie prędkości 0,5–2×.
- Napisy: czcionka, kolor, styl (obrys, cień, pole), położenie, pogrubienie, wymuszanie stylu ASS, wczytywanie z folderu filmu.
- Karta „Szczegóły”: rozdzielczość, kodek, klatki, dekodowanie, barwy/HDR, przepływność, pominięte klatki, bufor, synchronizacja A/V.
- Mysz na obrazie: dwuklik przy krawędzi przewija o 10 s, kółko – głośność albo przewijanie; zrzut ekranu bez napisów (Shift+S).
- **Kolor przewodni wszędzie:** do wyboru zasięg – same akcenty, także ikony, gałki i znaczniki (zamiast bieli) albo cały interfejs z tłami i tytułami. Własny kolor szesnastkowo i więcej gotowych barw.
- **Tło z okładki:** rozmyta okładka albo gradient jej barw jako subtelna nakładka pod całym oknem, z regulowanym kryciem i opcjonalnym powolnym ruchem.
- Wygląd: siła poświaty, zaokrąglenia, wielkość tekstu (90–125%), czcionka tytułów (szeryfowa, bezszeryfowa, techniczna).
- **Utwory jako okładki:** listy odtwarzania i biblioteka jako siatka kafelków z okładkami (grający utwór podświetlony, ulubione, kolejka, odtwarzanie z kafelka), wielkość z suwaka nad listą albo Ctrl+kółkiem.
- **Kompaktowy interfejs:** niższe wiersze list i panelu nawigacji, mniejsze odstępy i okładka – więcej utworów na ekranie.
- Nowa strona ustawień „Napisy” i rozbudowana strona „Wideo” (widok, obraz, sterowanie, dźwięk filmów, sieć: yt-dlp, jakość, bufor).

## 1.2.7

- Tryb wyłączny reaguje na głośność Windows (i wyciszenie) – program stosuje ją sam, gdy karta nie robi tego sprzętowo.
- Taśma utworu: 11 stylów (m.in. gęste słupki, lustro, diody, słupki 3D, fala 3D, linia, kropki), gradienty, poświata i znacznik z kropkami, podgląd miejsca pod kursorem, a przy znaczniku żywe słupki poruszające się z muzyką.
- Nowe wizualizacje: lustro, okrąg, widmo 3D, diody, stereo (Lissajous), kropki.
- Tekst piosenki obok tytułu w szerokim oknie – wiersz po wierszu przy znacznikach czasu, ręcznie przewijany bez nich; karta „Tekst” w panelu znika wtedy.
- Zwijane sekcje panelu nawigacji (zapamiętywane), logo na środku, przycisk chowania panelu i przywracania go z paska tytułu.
- Przycisk „Pokaż plik w Eksploratorze” przy bieżącym utworze.

## 1.2.6

- Podpowiedź po najechaniu na każdym przycisku z ikoną (zamykanie, poprzedni/następny w wideo, wyciszanie, usuwanie z kolejki, czyszczenie pola).
- Menu wyboru urządzenia i wyłącznika czasowego otwiera się tuż nad przyciskiem, a nie w połowie okna.

## 1.2.5

- Foldery w panelu rozwijane jak w Eksploratorze: strzałka (albo dwuklik) pokazuje podfoldery pod spodem, z wcięciem.
- Widok folderu pokazuje tylko pliki leżące bezpośrednio w nim, bez kafelków podfolderów.

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
