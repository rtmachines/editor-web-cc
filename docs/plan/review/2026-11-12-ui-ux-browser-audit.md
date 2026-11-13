# Audyt UI/UX — przekazanie do Claude Code

Data: 2026-11-12.
Cel: usunięcie ustaleń audytu bez rozszerzania zakresu o przebudowę wizualną aplikacji.
Status: raport, bez implementacji poprawek.

## Kontekst i wiarygodność dowodów

- Audyt obejmował analizę dokumentacji, szablonów, CSS, obsługi żądań i wybranych testów.
- Przegląd statyczny dotyczył drzewa roboczego na gałęzi `feat/machine-data-block-number`, z niezacommitowanymi zmianami.
- HEAD podczas analizy: `4bd52e856d6e5f2abcb9be637307e904b9a7e900`. To nie jest identyfikator audytowanego pliku wykonywalnego.
- Próby przeglądarkowe wykonano w Chromium przez `playwright-cli`.
- Uruchomiono istniejący `target/debug/examples/dev_server` na `127.0.0.1:18080`. Nie kompilowano kodu.
- Serwer używał danych w pamięci oraz własnego, tymczasowego hasła. Po audycie został zatrzymany.
- Dane audytu: maszyna `M901`, receptura `UX-901`, wersja 1, parametr `TEMP` typu `INT`, wartość 25, limity 0–100.
- Administrator tworzył i edytował dane. ProcessEngineer zatwierdzał. Operator sprawdzał odczyt i eksport.
- Plik wykonywalny nie zawierał nowego pola numeru DB w formularzu maszyny. Nie potwierdzono jego zgodności z aktualnym drzewem roboczym.
- Numery linii wskazują źródła odczytane podczas audytu. Przed poprawką należy odnaleźć aktualną implementację i ponownie sprawdzić zachowanie.
- Nie zmieniano kodu aplikacji. Ten raport jest jedyną zmianą wykonaną na późniejsze polecenie zapisania ustaleń.

Nie należy traktować wcześniejszego raportu statycznego i tego raportu jako dwóch zestawów ustaleń. F1–F5 to te same, zaktualizowane ustalenia.

## Instrukcja dla implementującego

1. Przeczytaj `AGENTS.md`, `CLAUDE.md`, reguły `.claude/rules/` i aktualne dokumenty normatywne.
2. Zweryfikuj każde ustalenie na aktualnej kompilacji. Uwzględnij trwającą zmianę `machine-data-block-number`.
3. Użyj grafu kodu do zawężenia zakresu, a następnie sprawdź źródła i testy.
4. Dla każdego potwierdzonego problemu dodaj test regresji, który zawodzi przed poprawką.
5. Zachowaj ochronę CSRF, kontrolę uprawnień, protokół `row_version`, kanoniczne wartości i zasady fokusu.
6. Jeżeli rozwiązanie wymaga zmiany polityki skryptów lub innej decyzji architektonicznej, przeprowadź wymagany proces ADR/OpenSpec.
7. Nie naprawiaj pozostałych niezwiązanych zmian drzewa roboczego w ramach tego audytu.
8. Raportuj osobno: odtworzenie, poprawkę, testy i pozostałe ograniczenia. Nie uznawaj niewykonanych prób za zaliczone.

Raport nie zastępuje wymaganych w repozytorium zatwierdzeń projektu lub zadań.

## F1 — critical — odpowiedź zapisu nadpisuje późniejszą edycję

Status: potwierdzone w przeglądarce dla edycji parametru i odpowiedzi 200.

### Źródła

- `crates/recipe-web/templates/parameter-form.html:5`
- `crates/recipe-web/src/pages/parameters/edit.rs:79`
- Analogiczny mechanizm: `crates/recipe-web/templates/translation-row.html:6`

### Problem i dowód

Formularz pozostaje edytowalny podczas żądania. `hx-sync="closest form:drop"` blokuje następne żądanie, ale nie wpisywanie.
Odpowiedź zapisu zastępuje formularz przez OOB stanem zapisanym na podstawie wcześniej wysłanych danych.

Próba:

1. W polu nazwy angielskiej wpisano `Sent before request`.
2. Kliknięto „Zapisz parametr” i wstrzymano oryginalne żądanie przeglądarki przez Playwright route.
3. Podczas oczekiwania wpisano `Typed while saving`.
4. Zwolniono żądanie przez `route.continue()`.

Wynik:

```text
HTTP: 200
Pole podczas oczekiwania: Typed while saving
Pole po odpowiedzi: Sent before request
Toast: Parametr TEMP został zapisany. Numer zmiany: 3.
Panel błędów: pusty
```

Pierwsza próba z `route.fetch()` została odrzucona przez CSRF. Nie jest dowodem tego ustalenia. Powyższy wynik pochodzi z powtórzenia z oryginalnym żądaniem.

### Wpływ i severity

Użytkownik traci później wpisany tekst mimo komunikatu sukcesu.
Poziom `critical` wynika z kategorii utraty danych w AGENTS.md. Dotyczy niezapisanej edycji formularza, nie uszkodzenia danych w bazie.

### Kierunek poprawki

Zachować zmiany powstałe po wysłaniu albo jawnie blokować edytowane kontrolki podczas zapisu.
Nie wystarczy zablokować samego przycisku zapisu. Zachować poprawny fokus i przywrócenie kontrolek po błędzie.
Nie usuwać kanonizacji wartości ani ochrony przed równoległymi zmianami.

### Kryteria akceptacji

- Opóźniona odpowiedź nie usuwa tekstu wpisanego po wysłaniu bez ostrzeżenia lub ochrony.
- Test obejmuje sukces 200 oraz błąd walidacji 422.
- Sprawdzić parametr i pojedynczy wiersz tłumaczenia.
- Sprawdzić zapis Enterem, kliknięciem oraz błąd transportu.

Odpowiedzi 422 i tłumaczenia nie zostały odtworzone w runtime. Ich ryzyko wynika z analizy statycznej.

## F2 — high — formularz parametru ma nieistniejący cel odpowiedzi

Status: potwierdzone w przeglądarce.

### Źródła

- `crates/recipe-web/templates/parameter-table.html:7`
- `crates/recipe-web/templates/version.html:46`
- `crates/recipe-web/templates/parameter-form.html:5`
- `crates/recipe-web/src/pages/versions/page.rs`, funkcja `add_href`

### Problem i dowód

Stan `after_end` nie renderuje `#parameter-rows`, ale nadal udostępnia link „Dodaj parametr”.
Formularz kieruje wysłanie do brakującego elementu.

Próba: otwarto draft `/recipes/UX-901/versions/1?page=2`, gdy lista mieściła się na jednej stronie.
Następnie otwarto „Dodaj parametr” i kliknięto „Pokaż pola wartości”.

```text
Żądania POST: 0
Konsola: htmx:targetError, #parameter-rows
Panel błędów: pusty
Liczba elementów #parameter-rows: 0
```

### Wpływ

Dostępna akcja nie działa i nie wyjaśnia problemu. Nieaktualny adres może pochodzić z zakładki lub wcześniejszego stanu listy.

### Kierunek poprawki

W stanie poza ostatnią stroną kierować do samodzielnego formularza albo zapewnić właściwy, stale obecny cel odpowiedzi.
Nie ograniczać poprawki do ukrycia komunikatu konsoli.

### Kryteria akceptacji

- E2E: nieaktualny numer strony → dodawanie → pokazanie pól → zapis.
- Brak `htmx:targetError`.
- Zapisany parametr jest widoczny po powrocie do listy.
- Zachować poprawne zachowanie pustej pierwszej strony i zwykłej paginacji.

## F3 — high — brak informacji po zerwaniu połączenia

Status: potwierdzone w przeglądarce dla przerwanego żądania zapisu parametru.

### Źródła

- `crates/recipe-web/templates/base.html:12`
- `crates/recipe-web/static/row-version.js:1`
- `crates/recipe-web/static/tree.js:1`
- `crates/recipe-web/static/htmx.min.js:1` — emituje zdarzenia błędu transportu
- `crates/recipe-web/templates/components/grid.html:6` — akcje zapisu bez wskaźnika oczekiwania

### Problem i dowód

Brakuje obsługi `htmx:sendError` i `htmx:timeout` w interfejsie aplikacji.
Panel odpowiedzi serwera nie pomaga, gdy odpowiedź nie dociera.

Próba: wpisano `Unsaved offline edit`, a żądanie zapisu przerwano przez `route.abort("connectionfailed")`.

```text
Panel błędów: pusty
Toast: pusty
Wartość pola: Unsaved offline edit
```

### Wpływ

Użytkownik nie zna wyniku operacji. Może opuścić stronę lub ponawiać zapis bez wiedzy, czy serwer go wykonał.
Brak widocznego stanu oczekiwania dodatkowo utrudnia rozpoznanie trwającego zapisu.

### Kierunek poprawki

Dodać wspólny komunikat błędu transportu oraz dostępny stan oczekiwania.
Komunikat utraty odpowiedzi powinien mówić o niepotwierdzonym wyniku, a nie zapewniać, że zapis nie nastąpił.
Rozwiązanie musi uwzględniać normatywną politykę skryptów i CSP. Nie modyfikować biblioteki vendored HTMX.

### Kryteria akceptacji

- Przerwanie transmisji daje widoczny i dostępny komunikat.
- Wpisane dane pozostają dostępne.
- Stan oczekiwania kończy się po sukcesie i błędzie.
- Test obejmuje zerwanie przed wysłaniem oraz utratę odpowiedzi po wykonaniu operacji.
- Zweryfikować ogłoszenie komunikatu czytnikiem ekranu. Tego nie wykonano podczas audytu.

## F4 — high — Enter w UDT ma nieprzewidywalną semantykę

Status: częściowo potwierdzone w przeglądarce. Granica 1024 wierszy pozostaje do odtworzenia.

### Źródła

- `crates/recipe-web/templates/udt-form.html:18`
- `crates/recipe-web/src/pages/types/define.rs:338`
- `crates/recipe-web/src/pages/types/form.rs` — stan `at_max`

### Problem i dowód

Pierwszym przyciskiem submit jest „Dodaj składnik”, a nie wizualnie główne „Utwórz typ”.
Po osiągnięciu limitu wierszy przycisk dodawania znika. Domyślnym przyciskiem staje się trwałe utworzenie typu lub rewizji.

Próba przeglądarkowa: w formularzu nowego UDT wpisano nazwę `ST_Audit` i naciśnięto Enter w tym polu.
Żądanie zawierało `add=1`, odpowiedź miała 200, a formularz dodał drugi składnik.

Nie odtworzono formularza z 1024 wierszami ani utworzenia rewizji Enterem przy tym limicie.

### Wpływ

Enter wykonuje pomocniczą akcję, a według źródeł przy limicie zmienia znaczenie na trwały zapis.

### Kierunek poprawki

Ustalić jednoznaczne, niezmienne działanie Enter. Rozdzielić dodawanie wiersza od domyślnego wysłania formularza.

### Kryteria akceptacji

- Test nowego typu i nowej rewizji.
- Enter w polu nazwy oraz typu składnika przy 1, 1023 i 1024 wierszach.
- Ten sam gest nie zmienia znaczenia wskutek zniknięcia przycisku.
- Nie powstaje niezamierzona rewizja.

## F5 — medium — rodzic jednostki nie aktualizuje kontekstu

Status: potwierdzone w przeglądarce.

### Źródła

- `crates/recipe-web/templates/unit-form.html:5`
- `crates/recipe-web/src/pages/units/form.rs:111`
- `crates/recipe-web/i18n/pl.ftl:369`

### Problem i dowód

Nagłówek i proponowany segment pochodzą od początkowego rodzica. Zmiana rodzica w select nie aktualizuje tych informacji.

Próba:

1. Otworzono „Dodaj podjednostkę do U1”.
2. Zmieniono rodzica na „Brak. Jednostka jest jednostką główną”.
3. Pozostawiono proponowany numer 1 i wysłano formularz.

Nagłówek nadal wskazywał „Nowa jednostka w U1”. Numer pozostał równy 1.
Serwer zwrócił `E-PLT-004`: „Jednostka U1 już istnieje na tej maszynie”.

### Wpływ

Formularz pokazuje sprzeczny kontekst i wywołuje zbędny błąd mimo podpowiedzi, że proponuje pierwszy wolny numer.

### Kierunek poprawki

Neutralny nagłówek albo aktualizacja kontekstu i propozycji po zmianie rodzica.
Zachować wpisaną nazwę i rozróżnić propozycję automatyczną od segmentu świadomie zmienionego przez użytkownika.

### Kryteria akceptacji

- Przełączanie między rodzicami o różnych zajętych segmentach.
- Przełączanie między podjednostką a jednostką główną.
- Nagłówek, propozycja i rzeczywisty cel zapisu pozostają zgodne.

## Priorytety i szacunki

Szacunki obejmują implementację i test regresji. Wymagają potwierdzenia po analizie rozwiązania.

| Kolejność | Ustalenie | Nakład |
|---|---|---|
| 1 | F1: nadpisywanie edycji podczas zapisu | 1–3 dni |
| 2 | F2: brak celu formularza | do 1 dnia |
| 3 | F3: obsługa transportu i oczekiwania | 1–3 dni, możliwa decyzja architektoniczna |
| 4 | F4: semantyka Enter | do 1 dnia |
| 5 | F5: kontekst rodzica | 0,5–2 dni |

## Wykonane kontrole pozytywne

- Autor nie mógł zatwierdzić własnej wersji. Interfejs wyjaśniał rozdzielenie obowiązków.
- ProcessEngineer zatwierdził wersję utworzoną i edytowaną przez Administratora.
- Operator nie widział przycisków edycji wersji i pobrał `recipe_UX-901_v1.json`.
- Tryb tylko do odczytu ukrył formularz tworzenia maszyny i pokazał przyczynę blokady.
- Przejście Tab na stronie wersji nie wykazało pułapki. Elementy interaktywne miały obrys fokusu i pozostawały w widoku.
- Pierwszy Tab na ekranie logowania wskazywał link „Przejdź do treści”.
- Zmiana języka na angielski zachowała stronę wersji i zmieniła `html lang` na `en`.
- Obejrzano ekran wersji w wymuszonych kolorach.
- Sprawdzono 11 ekranów przy 320, 768 i 1280 px. Wszystkie 33 pomiary dały szerokość dokumentu równą szerokości viewportu.
- Prosta kontrola pól na tych ekranach nie znalazła elementów bez label, aria-label lub aria-labelledby. Nie jest to pełny test dostępności.
- W analizie statycznej przeliczono 59 par kontrastu z systemu projektowego. Wszystkie spełniały zadeklarowane progi.

Ekrany objęte pomiarami szerokości: receptury, maszyny, szczegóły maszyny, edycja połączenia PLC, nowy UDT, użytkownicy, audyt, profil, historia receptury, zatwierdzanie i edycja parametru.

## Pozostałe luki i hipotezy

- Nie wykonano pełnego axe ani próby czytnikiem ekranu. Brak podstaw do potwierdzenia pełnej zgodności WCAG.
- Nie wykonano pełnych ścieżek odrzucania, wycofywania, usuwania, przywracania i resetowania haseł. Ta runda skupiała się na F1–F5 i głównej ścieżce zatwierdzania.
- Nie wykonano konfliktu dwóch równoległych edytorów. Mechanizm sprawdzono wcześniej statycznie.
- Zapis połączenia PLC i nowy numer DB wymagają aktualnej kompilacji. Obejrzano formularz połączenia starszego pliku wykonywalnego.
- Nie sprawdzono granicy 1024 składników UDT w przeglądarce.
- Nie potwierdzono F1 dla odpowiedzi 422 i tłumaczeń w runtime.
- Nie wykonywano Cargo, Clippy/reviewdog ani pełnej istniejącej suity E2E.
- Z użytkownikami warto sprawdzić częstotliwość dalszego pisania po zapisie, rzeczywiste opóźnienia oraz potrzebę edycji na wąskich ekranach.
- Warto zweryfikować czytelność katalogu wielu rewizji UDT i zrozumienie usuwania składnika przez wyczyszczenie dwóch pól.
- Wyścig eksportu przez `/approved` jest zaakceptowany w ADR-0052. Nie zgłoszono go jako błędu implementacji.

## Materiały pomocnicze

Pliki poniżej powstały w `/tmp`. Są lokalne i nietrwałe. Nie zakładać ich obecności w innym środowisku. Treść raportu zawiera dowody potrzebne do ponownego odtworzenia.

- `/tmp/ux-login-desktop.png`
- `/tmp/ux-login-320.png`
- `/tmp/ux-F2.png`
- `/tmp/ux-parameter-320.png`
- `/tmp/ux-approve-1280.png`
- `/tmp/ux-forced-colors.png`
- `/tmp/ux-export.json` — wyłącznie dane audytowe

Nie kopiować do zgłoszeń plików haseł, cookies ani stanów sesji przeglądarki.
