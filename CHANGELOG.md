# Zmiany

## 2.0.7

- „Popraw dopasowanie TMDB” pojawia się w menu tylko przy filmach i serialach, przy muzyce już nie.

## 2.0.6

- Tryb bezczynności zostawia okładkę, tytuł, tekst piosenki i tło. Znikają tylko panel nawigacji, listy, panel po prawej i dolny pasek, a wizualizacja zajmuje ich miejsce pod okładką na całą szerokość okna.

## 2.0.5

- Tryb bezczynności liczy tylko bezczynność samego programu: ruch kursora nad oknem i klawisze lub kliknięcia, gdy okno ma fokus. Praca w innych programach nie przerywa odliczania.
- Kafelki: liczba odtworzeń i czas trwania bez tła i obramowania; tytuł bez wykonawcy schodzi na dolną linię.

## 2.0.4

- Tryb bezczynności: po 90 sekundach bez myszy i klawiatury, gdy gra muzyka, a wizualizacja jest w tle, interfejs przez około minutę powoli znika, a wizualizacja rośnie na całe okno. Kliknięcie w okno albo dowolny klawisz przywraca zwykły widok.
- Teksty piosenek: program najpierw szuka wersji z sygnaturami czasu – dokładne dopasowanie, bez albumu, z tytułem bez dopisków („Remastered”, „Live”, „feat.”), z pierwszym wykonawcą, na końcu wyszukiwanie z najbliższym czasem trwania. Sam tekst tylko wtedy, gdy nigdzie nie ma wersji z czasami.
- Gdy plik albo tag ma sam tekst bez czasów, program pokazuje go od razu i w tle szuka wersji z czasami; po znalezieniu ją podmienia.
- Wcześniej zapamiętane wyniki z LRCLIB są pobierane jeszcze raz, żeby znaleźć wersje z czasami.

## 2.0.3

- Kafelki utworów i filmów: czas trwania zawsze w prawym górnym rogu okładki, a liczba odtworzeń w lewym górnym.
- Wysokość podglądu nad listą zmienia się przeciąganiem widocznego separatora, a nie całego odstępu nad przyciskami.
- Przycisk sortowania ma swoją szerokość i nie rozciąga się na pasek.
- Napis „Teraz odtwarzane” jest teraz plakietką w kolorze akcentu z czytelniejszym krojem, a miejsce na liście stoi obok.

## 2.0.2

- Wizualizacja w tle sięga dokładnie do tych samych krawędzi co kafelki i listy, także przy otwartym panelu po prawej.

## 2.0.1

- Naprawione: wizualizacja w tle schodziła pod pasek odtwarzania. Teraz zawsze kończy się nad nim.

## 2.0.0

- Szybsze wczytywanie okładek z plików FLAC, zwłaszcza z dysku sieciowego: okładka czytana jest prosto z bloków metadanych na początku pliku, resztę program przeskakuje. Pozostałe formaty czytają same tagi, bez właściwości dźwięku.
- Więcej równoległych odczytów okładek (8–16 wątków): przy dysku sieciowym większość czasu to czekanie na sieć.
- Dużo szybsze wczytywanie okładek: gotowe miniatury zapisywane są na dysku, więc przy kolejnym uruchomieniu nie trzeba ponownie czytać tagów ani dekodować dużych obrazów. Okładki z sieci (MusicBrainz, 1 zapytanie na sekundę) mają osobną kolejkę i nie blokują okładek z dysku. Więcej wątków wczytujących, a listy i kafelki odświeżane są zbiorczo zamiast po każdej okładce.
- Wizualizacja w tle zawsze kończy się nad paskiem odtwarzania.
- Wydania 1.14.2 i 1.14.3 nie zbudowały się, więc 2.0.0 zawiera też ich zmiany: nowy panel boczny, kolekcje AI, wizualizację w tle z rozmyciem i do 512 słupków, zawijanie tekstu piosenek i napisów oraz pasek przewijania panelu nawigacji przy krawędzi.

## 1.14.3

- Za długie linie tekstu piosenki i napisów filmu zawijają się na kolejny wiersz zamiast urywać się wielokropkiem – obok opisu utworu i w panelu tekstu.
- Analizator może mieć do 512 słupków (wcześniej 256).
- Wizualizacja w tle jest wyśrodkowana względem treści okna, bez panelu nawigacji, i nie wchodzi pod niego.
- Nowe ustawienie: rozmycie wizualizacji w tle (Ustawienia → Wizualizacja).
- Pasek przewijania panelu nawigacji leży przy jego krawędzi, a nie w odstępie od niej.

## 1.14.2

- Nowy wygląd panelu po prawej (tekst, informacje, kolejka), podobny do panelu nawigacji: gradient od krawędzi okna, karty z ikonami, wiersze z ikonami; panel dochodzi do krawędzi okna.
- Panel filmu na pełnym ekranie dostał ten sam gradient, odstępy i zaokrąglenia kart.
- Mniejsze zużycie pamięci i karty graficznej przy filmach: bufor odczytu tylko dla sieci, lżejsza domyślna jakość skalowania, rzadsze odświeżanie poświaty, analizator wyłączony podczas filmu.

## 1.14.1

- Suwak widoku (lista – okładki) jest w każdym widoku biblioteki, także przy kafelkach: gatunkach, latach, albumach, wykonawcach, serialach i kolekcjach AI.
- Przycisk wyboru kolumn przeniesiony na początek nagłówka listy – nie zasłania już nazwy ostatniej kolumny.
- Nowi dostawcy AI: OpenRouter, Google Gemini, Mistral, Groq, DeepSeek, xAI i lokalna Ollama (bez klucza); po zmianie dostawcy model wybiera się od nowa.
- Nagłówki sezonów seriali mają okładkę (plakat serialu albo klatkę pierwszego odcinka).
- Wizualizacja w tle nie wchodzi już na panel nawigacji ani dolny pasek: jest wyśrodkowana w obszarze po prawej. Słupki stoją na dole, a tryby symetryczne (lustro, okrąg, fala, plama, puls) są na środku.
- Analizator może mieć do 256 słupków (wcześniej 96).

## 1.14.0

- **AI (opcjonalnie, z własnym kluczem API)**: nowa strona Ustawienia → AI – dostawca (Anthropic, OpenAI albo serwer zgodny z API OpenAI, np. OpenRouter, LM Studio, Ollama), klucz, model wybierany z listy pobranej od dostawcy i temperatura.
- Analiza biblioteki uruchamiana ręcznie: model rozpoznaje język, nastrój, tematy i energię utworów. Wysyłane są tylko metadane (wykonawca, tytuł, album, gatunek, rok); analizę można zatrzymać, powtórzyć albo usunąć jej wyniki.
- Sekcja **Kolekcje AI** w panelu (tylko gdy AI jest ustawione): Języki, Nastroje, Tematy i Energia, a przyciskiem „+” – listy z opisu („spokojne piosenki o morzu na wieczór”). Model zamienia opis na filtr, utwory dobierane są lokalnie z biblioteki.

## 1.13.2

- Wizualizację można pokazywać na okładce albo w tle całego okna (Ustawienia → Wizualizacja → Pokazuj wizualizację).
- Rodzaj wizualizacji wybiera się w ustawieniach; przełącznik na okładce usunięty.
- Naprawione: kolory wizualizacji, przyciemnienie okładki, FPS i szybkość animacji zmienione w ustawieniach nie działały w oknie programu.
- Wyraźny separator między panelem bocznym a resztą okna. Panel ma też ciemniejsze tło – gradient od lewej krawędzi okna – więc jest dobrze widoczny na każdym tle.
- Przycisk zamknięcia pliku przeniesiony z okładki na koniec rzędu ikon pod tytułem (serce, informacje, folder, kolejka).
- Ruch tła to płynny, rozmyty gradient barw okładki zamiast wyraźnych plam światła.

## 1.13.1

- Ustawienie „Zaokrąglenia” obejmuje też dużą okładkę, kafelki, menu, okna dialogowe, panele filmu i inne elementy; okrągłe przyciski i pigułki zostają okrągłe.
- Tło z okładki (rozmyta okładka, gradient barw, ruch tła) działa także przy przezroczystym oknie – delikatne barwy okładki nad tym, co prześwituje zza okna.

## 1.13.0

- Nowe wizualizacje: Fala, Plama, Puls i Kapsułki.
- Kolory wizualizacji do wyboru: odcienie akcentu, barwy okładki albo kolorowe; są kryjące, więc widać je także przy przezroczystym oknie.
- W ustawieniach wizualizacji: siła przyciemnienia okładki pod wizualizacją (także wyłączone), liczba klatek na sekundę (15–60) i szybkość animacji.
- Nowy przełącznik wizualizacji na okładce: szklana pigułka ze strzałkami poprzednia / następna.
- Panel boczny bez własnego tła – przezroczysty jak reszta okna.
- Znacznik odtwarzania na taśmie to sama linia, bez kropek na końcach.

## 1.12.3

- Po cichej aktualizacji OpenMP uruchamia się znowu jako zwykły użytkownik, a nie administrator – wracają dyski sieciowe, odtwarzanie i okładki.
- Gdy kilku plików z rzędu nie da się odtworzyć, odtwarzanie się zatrzymuje z komunikatem, zamiast przeskakiwać przez całą kolejkę.
- Nową kolekcję z filtrów tworzy przycisk „+” przy nagłówku Kolekcje.

## 1.12.2

- Okno aktualizacji pokazuje się zawsze na ekranie – także gdy okno programu jest zminimalizowane albo schowane w zasobniku – a po instalacji OpenMP uruchamia się ponownie w nowej wersji.
- „Pokaż w Eksploratorze” zaznacza plik w jego folderze, zamiast otwierać Dokumenty (ścieżki ze spacjami).
- Wyszukiwanie w sekcji Muzyka pokazuje tylko muzykę, a w sekcji Wideo tylko filmy; z kolekcji, list odtwarzania i folderów – obie.
- Podczas odtwarzania wideo od razu działają strzałki i inne skróty klawiatury.

## 1.12.0

- **Kolekcje**: zbiory i inteligentne listy połączone w jedną sekcję panelu. Nowe kolekcje: Nowe odkrycia (pierwszy raz grane w ostatnich 30 dniach), Dawno niegrane, Nigdy niegrane, Długie utwory, Wysoka jakość (bezstratne i hi-res), Brakujące tagi i Obejrzane filmy. Własne kolekcje z filtrów tworzy pozycja „Nowa kolekcja z filtrów…”.
- Puls taśmy przy znaczniku przesuwa się płynnie jak linia odtwarzania, zamiast przeskakiwać o kolumnę.

## 1.11.0

- **Prawdziwe szkło**: okno programu, okna ustawień i dialogów oraz menu używają akrylu Windows 11 – prześwituje rozmyte to, co jest za oknem, zamiast podkładanej okładki. Przezroczystość ustawia suwak „Szkło: przezroczystość”.
- Ujednolicona nazwa: wszędzie „kolor akcentu” (wcześniej miejscami „kolor przewodni”).
- Gdy kolor akcentu jest brany z okładki, wybór koloru i pole własnego koloru są ukryte; przełącznik „z okładki” jest na początku sekcji.

## 1.10.0

- **Sortowanie w każdym widoku listy**: przełącznik obok filtrów pokazuje, jak lista jest ułożona; klik otwiera wybór (tytuł, wykonawca, album, rok, długość, data pliku, liczba odtworzeń, ostatnio grane, ocena, rozmiar). Każdy widok ma swoje domyślne – np. „Ostatnio dodane” wg daty pliku, „Najczęściej odtwarzane” wg liczby odtworzeń – a wybór jest zapamiętany.
- Nad siatką i listą nie ma już wyśrodkowanego tytułu i podtytułu widoku – jest zaznaczony w panelu nawigacji.
- Rząd okładek wjeżdżający pod górną krawędź stopniowo znika.
- **Okładki wczytują się szybciej**: najpierw te, które są właśnie na ekranie, i w czterech wątkach zamiast dwóch.
- Naprawiona żywa część taśmy przy znaczniku (w 1.9.6 była niewidoczna).

## 1.9.6

- Poprawka budowania: miękkie krawędzie żywej części taśmy (wydania 1.9.0–1.9.5 nie zbudowały się).

## 1.9.2

- Strony ustawień bez dużego nagłówka z nazwą strony (jest zaznaczona na liście po lewej).
- Strona „Napisy” podzielona na dwie opisane części: napisy na obrazie filmu (w oknie i na pełnym ekranie) oraz tekst piosenki i napisy w panelu.

## 1.9.1

- Przycisk przypinania na wierzchu stoi w jednej grupie z przyciskami okna (minimalizuj, maksymalizuj, zamknij) i wygląda jak one – w oknie głównym i w widoku filmu.

## 1.9.0

- **Kolumny do wyboru w każdym widoku listy**: wykonawca, album, gatunek, rok, format, rozmiar, liczba odtworzeń, ocena, ostatnio grane, data pliku, kompozytor i folder. Nad listą pasek z nazwami kolumn – prawy klik albo przycisk na jego końcu otwiera wybór. Osobny zestaw dla list odtwarzania, folderów, albumów, zbiorów, filmów i seriali, zapamiętany.
- Kolumny listy trzymają równy pion we wszystkich wierszach (wcześniej długi tytuł przesuwał resztę, np. „Sezon · Odcinek” w serialach).
- Żywa część taśmy przy znaczniku zaczyna się i kończy łagodnym przejściem zamiast ostrej krawędzi.

## 1.8.0

- **Korektor rozbudowany**: krzywa charakterystyki rysowana na żywo za suwakami (wypadkowa pasm, barwy i wzmocnienia, czerwona powyżej 0 dB), nowe suwaki basów (100 Hz) i sopranów (10 kHz) – także w filmach, ostrzeżenie o przesterowaniu z przyciskiem „Dobierz wzmocnienie” i usuwanie własnych ustawień.
- **Filtr w zbiorach**: w ulubionych, ocenionych, najczęściej odtwarzanych, ostatnio dodanych, historii i duplikatach obok wyszukiwania jest przełącznik Wszystko / Muzyka / Filmy (zapamiętywany). „Ostatnio dodana muzyka” to teraz „Ostatnio dodane”.
- **Ładniejsze suwaki**: ścieżka grubnie pod kursorem, wypełnienie z gradientem akcentu, jasny uchwyt z obwódką. Głośność w kapsule z dymkiem wartości przy przeciąganiu i obsługą kółka myszy.

## 1.7.3

- **Wygląd tekstu piosenki w ustawieniach** (Napisy → „Tekst piosenki i napisy w panelu”): czcionka, wielkość, powiększenie śpiewanej linii, odstęp między liniami, osobne kolory śpiewanej i pozostałych linii, wyrównanie, pogrubienie i przygaszanie dalszych linii – z podglądem. Jedno ustawienie dla muzyki i filmów, przy okładce i w panelu bocznym.
- **Fala utworu na pasku przewijania** pokazuje głośność (RMS) w skali dopasowanej do każdego utworu – koniec z płaską taśmą przy głośno masterowanych piosenkach.
- Żywa część taśmy przy znaczniku sunie płynnie zamiast skakać o kolumnę.
- Cienkie paski przewijania w kolorach motywu we wszystkich listach, siatkach i ustawieniach.
- Widoki biblioteki, listy, radia i statystyk bez dodatkowego tła i obrysu.
- Nagłówki stron i okien wyśrodkowane; strzałka przy nagłówku sekcji w panelu nawigacji wyrównana z tekstem.
- Z paska odtwarzania zniknęły przyciski panelu nawigacji i „Tekst, informacje, kolejka” (panel boczny dalej pod klawiszem L).

## 1.7.2

- **Kolejka uporządkowana**: karta „Kolejka” to teraz bieżące odtwarzanie – granie z folderu, serialu czy albumu zastępuje jej zawartość (od klikniętego pliku), zamiast doklejać kolejne foldery. „Następny” to kolejny plik z folderu albo kolejny odcinek. Pozycje dodane ręcznie przez „Dodaj do kolejki” (jeszcze niezagrane) zostają i grają w swojej kolejności. Film kliknięty we „Wszystkich filmach”, ostatnio dodanych czy wynikach wyszukiwania gra sam.
- Program otwiera się na tym samym widoku, na którym go zamknięto – ta sama lista, folder, serial czy album.
- Miejsce w filmie („Kontynuuj oglądanie”) zapisuje się co minutę w trakcie oglądania, nie tylko przy pauzie i zmianie pliku – nie przepada po awarii czy wyłączeniu komputera.
- Historia pomija pliki odtwarzane krócej niż 5 sekund.

## 1.7.1

- Sterowanie filmu nie chowa się, gdy kursor jest na pasku przewijania albo trwa wybieranie momentu (także przy przeciąganiu poza pasek).

## 1.7.0

- **Ustawienia uporządkowane**: wspólne dla muzyki i filmów w „Odtwarzaniu”, muzyka w „Dźwięku”, wszystko o filmach w „Wideo” – każda strona podzielona na sekcje.
- **Napisy z plików MP4/M4V/MOV** (mov_text) trafiają do paska tekstu od razu w całości, jak z MKV i plików SRT – można przewinąć listę i kliknąć dowolną kwestię, by przeskoczyć film.
- **Seriale**: nowa pozycja w filmach biblioteki – kafelki seriali (rozpoznawanych po S01E02/1x02 w nazwie pliku), a po kliknięciu strona serialu z odcinkami podzielonymi na sezony. Podwójne kliknięcie odcinka odtwarza go i kolejne.
- **Wysokość górnego panelu** (podgląd utworu/filmu) zmienia się przeciągnięciem granicy nad listą; wysokość jest zapamiętywana, podwójne kliknięcie przywraca automatyczną.
- Bez plakietki „W kolejce” na kafelkach i wierszach.
- Podwójne kliknięcie filmu w bibliotece (np. we „Wszystkich filmach”) dodaje do kolejki tylko ten film, a nie cały widok.
- Na filmie większy tytuł, bliżej górnej krawędzi; czasy nad paskiem kontrolek wyrównane do ikon pod spodem.
- **Rozsądniejszy dobór napisów**: przy polskiej ścieżce dźwiękowej napisy są wyłączone; bez niej włączają się polskie, a gdy ich nie ma – angielskie (kolejność z listy preferowanych języków napisów). Wybór zapamiętany dla filmu ma pierwszeństwo.

## 1.6.4

- Ekran powitalny pokazuje się tylko przy pierwszym uruchomieniu (bez pliku ustawień) – nigdy przy już skonfigurowanym programie.

## 1.6.3

- **Ekran powitalny** przy pierwszym uruchomieniu: wybór języka, foldery z muzyką i filmami (także dyski sieciowe), ikona w zasobniku i skojarzenia plików. Skanowanie biblioteki rusza po kliknięciu „Zaczynamy”. Kto aktualizuje starszą wersję z biblioteką, go nie zobaczy.
- Wydania zawierają już tylko instalator (bez wersji przenośnej ZIP).

## 1.6.2

- **Odtwarzanie na urządzeniach DLNA** (menu → Odtwórz na urządzeniu): wyszukanie telewizorów, amplitunerów i głośników sieciowych w sieci lokalnej i wysłanie bieżącego utworu albo filmu – lokalny plik przez wbudowany serwer (z przewijaniem), strumień wprost; zatrzymanie z tego samego menu.
- **Płyty CD audio** (menu → Odtwórz płytę CD): odtwarzanie z pierwszego napędu CD/DVD przez silnik wideo; utwory płyty są rozdziałami (PgUp/PgDn).
- Odcinki podcastów dłuższe niż 20 minut wznawiają się od miejsca przerwania (jak filmy).
- **Podcasty** (panel biblioteki → Podcasty → Dodaj podcast…): subskrypcja z adresu kanału RSS, lista odcinków z datą i długością, odtwarzanie jak strumień; prawy przycisk – sprawdzenie nowych odcinków albo usunięcie subskrypcji.
- **Własne skróty klawiszowe** (Ustawienia → Skróty klawiszowe): 20 akcji – odtwarzanie, przewijanie, głośność, losowo, powtarzanie, ulubione, kolejka, korektor, pełny ekran, miniodtwarzacz i inne. Kliknij pole i naciśnij kombinację; Backspace usuwa. Własne skróty mają pierwszeństwo przed domyślnymi.

## 1.6.1

- 1.6.0 nie zbudowało się (brak importu w siatce okładek) – ta wersja zawiera wszystkie jej zmiany.
- **Ikona w zasobniku systemowym** (Ustawienia → System): zamknięcie okna chowa je do zasobnika, a muzyka gra dalej; kliknięcie przywraca okno, prawy przycisk – menu (odtwórz/pauza, następny, poprzedni, zakończ); podpowiedź pokazuje bieżący utwór.
- **Kopia zapasowa** (Ustawienia → System): eksport i import ustawień, list, ulubionych, ocen i statystyk w jednym pliku – także na inny komputer; po imporcie program uruchamia się ponownie.
- **Duplikaty** w panelu biblioteki (Zbiory): utwory z tym samym wykonawcą i tytułem (bez dopisków w nawiasach) i podobną długością, obok siebie – do przejrzenia i usunięcia.
- **Raport awarii:** gdy program zamknie się nieoczekiwanie, przy następnym uruchomieniu pokazuje plik z raportem (crash-last.txt), który można wysłać autorowi.

## 1.6.0

- **Wersja angielska:** cały interfejs po angielsku i po polsku; domyślnie język Windows, zmiana w Ustawieniach → Wygląd → Język.
- **Łatwe dodawanie języków:** wszystkie teksty jednego języka w jednym pliku lang/<kod>/LC_MESSAGES/openmp.po (szablon lang/openmp.pot), wbudowywanym przy kompilacji – bez zmian w kodzie. Liczba mnoga i separator dziesiętny zgodnie z językiem.
- **Podpisy na okładkach** (Ustawienia → Układ): tytuł i podtytuł na dole okładki zamiast pod nią – więcej okładek na ekranie.
- Radio „Mój kraj” według ustawień regionalnych Windows; instalator także po angielsku.

## 1.5.5

- **Okładka po prawej, tekst po lewej:** okładka, plakat albo obraz filmu stoi przy prawej krawędzi w prawdziwych proporcjach (pionowy plakat, kwadratowa płyta, szeroki kadr) – bez przycinania i bez zmiany układu tekstu.
- **Ostrzejsze okładki:** zaznaczenie wczytuje okładkę w pełnej rozdzielczości (do 900 px) zamiast miniatury z listy; okładka odtwarzanego pliku też do 900 px; plakaty z TMDB w rozmiarze 780 px.

## 1.5.4

- **Obejrzany i odsłuchany od progu** (Ustawienia → Odtwarzanie, domyślnie 80%): odtworzenie liczy się w statystykach dopiero po przejściu tej części długości, a film dostaje znacznik „obejrzany” od tego samego progu. Pominięte po chwili utwory nie nabijają odtworzeń.
- **Zaznaczenie wygląda jak „Teraz odtwarzane”:** ten sam nagłówek, szeryfowy tytuł i wykonawca, album i rok (film – rok, gatunki, ocena) z ikoną, twórcy, opis i parametry techniczne z ikonami. Niżej tylko dodatkowe tagi – dla muzyki m.in. wykonawca albumu, numer, gatunek.

## 1.5.3

- **Naprawione okładki:** każdy plik ma własną okładkę – utwory z różnych albumów w jednym folderze nie dostają już okładki pierwszego z nich.
- **Ciemny motyw przy jasnych okładkach:** okładka bez wyraźnych barw (czarno-biała, szara) daje akcent w kolorze szampańskiego złota zamiast bieli; tekst na przyciskach i znacznikach w kolorze akcentu ciemnieje, gdy akcent jest jasny – „Odtwórz” znów jest czytelne.
- **Bardziej elegancki wygląd:** szeryfowe nagłówki biblioteki i albumów, okładki z cienką krawędzią światła, kafelki unoszące się z poświatą po najechaniu zamiast szarego tła.
- **Ciaśniejszy widok odtwarzania:** mniejsze pionowe odstępy w opisie utworu, zaznaczeniu i dolnym pasku.

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
