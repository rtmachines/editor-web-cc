# Głęboki przegląd repozytorium — raport dla Claude Code

Data przeglądu i przekazania: 2026-11-11.
Język: polski, zgodnie z poleceniem użytkownika.
Autor: Codex, przegląd z użyciem bieżącego modelu rozmowy.

## 1. Wynik i zasady przekazania

Potwierdzono **6 ustaleń: 1 high, 3 medium, 2 low**. Nie potwierdzono ustalenia critical.

Poziom high w R-01 wynika z naruszenia twardej reguły repozytorium. Nie oznacza wykazanego przepełnienia, wycieku, utraty danych ani awarii dostępności. W pozostałych ustaleniach również rozdzielono obecny błąd od ryzyka przyszłej regresji.

Przegląd produkcyjnego kodu Rust i wskazanych przepływów został domknięty. **Przegląd całego uzgodnionego korpusu pozostaje częściowy.** Nie zakończono pełnego semantycznego przeglądu wszystkich testów, pomocników i dokumentów historycznych ani pełnej macierzy scenariusz specyfikacji → asercja testu.

Ten plik przekazuje wyniki, a nie wdrożone poprawki. Podczas przeglądu nie zmieniano plików ani stanu Git. Sam raport zapisano później, na wyraźne polecenie użytkownika. Nie wykonano żadnego polecenia cargo, instalacji, testu mutacyjnego, uruchomienia środowiska produkcyjnego ani publikacji wyników.

Wskazówki dla kolejnego agenta:

- Zachowaj identyfikatory R-01–R-06 w planie napraw i odpowiedziach na ustalenia.
- Przed zmianą sprawdź bieżący HEAD, lokalne zmiany, AGENTS.md, CLAUDE.md i aktualne ADR. Lokalizacje tego raportu dotyczą rewizji z sekcji 3.
- Nie traktuj luk dowodowych z sekcji 8 jako potwierdzonych usterek produkcyjnych.
- R-03 wymaga rozstrzygnięcia sprzecznej gwarancji przez właściciela. Raport nie wybiera jednej interpretacji.
- Nie zmieniaj zaakceptowanych wyjątków i ograniczeń architektury tylko dlatego, że przypominają ogólny wzorzec ryzyka.
- Statyczny odczyt testu nie oznacza jego przejścia. Wyniki CI opisano osobno i powiązano z konkretnym HEAD.

## 2. Potwierdzone ustalenia

### R-01 — high — Arytmetyka nasycająca poza dozwolonymi licznikami UI

**Lokalizacje:**

- `crates/recipe-server/src/limited_read.rs:94` — `marks.saturating_sub(1)`.
- `crates/recipe-server/src/limited_read.rs:107` — `marks.saturating_add(1)`.
- `crates/recipe-server/src/credential_cipher.rs:94` — `HEADER_LENGTH.saturating_add(encrypted.len())`.
- Dodatkowe potwierdzone wystąpienie tej samej niezgodności: `crates/recipe-server/src/create_admin.rs:63`. Dodawanie do stałej 1024 nie może tu osiągalnie przepełnić zakresu, ale również nie jest licznikiem UI.

**Wymaganie:** `.claude/rules/rust.md:12` oraz `docs/architecture/reliability.md:233` wymagają `checked_*` i obsługi `None` jako błędu. `saturating_*` jest dopuszczone wyłącznie dla liczników pokazywanych przez UI, gdzie nasycenie jest prawidłowe. `AGENTS.md:33` klasyfikuje naruszenie twardej reguły jako high.

**Błąd i warunki:** licznik zawieszonych odczytów steruje dopuszczeniem kolejnych operacji dyskowych. Nie służy prezentacji UI. Obliczanie pojemności bufora szyfrogramu także nie mieści się w wyjątku. Naruszenie jest obecne niezależnie od osiągnięcia granicy typu całkowitego.

**Dowód i zabezpieczenia:** pełny kontekst `Call`, `FinishOnDrop` i `read_with_limit` został przeczytany. Zmiany oznaczeń są synchronizowane mutexem. W obecnych produkcyjnych wywołaniach nie wykazano osiągalnego underflow ani overflow. ADR-0133 i ADR-0151 nie ustanawiają wyjątku od reguły arytmetyki.

**Skutek:** kod nie spełnia obowiązującej polityki obsługi błędów arytmetycznych. Nasycenie ukryłoby przekroczenie zakresu zamiast je obsłużyć. Nie potwierdzono z tego powodu obecnej utraty danych, ujawnienia sekretu lub awarii.

**Zakres poprawki:** zastosować sprawdzaną arytmetykę z określoną obsługą błędu albo reprezentację oznaczeń niewymagającą licznika. Zachować bezpanikową ścieżkę `Drop`. Nie zastępować nasycania operacją niekontrolowaną ani `unwrap`.

**Weryfikacja:** zachować testy dwóch oznaczeń, zakończenia operacji na granicy timeoutu, odczytu bez własnego oznaczenia i paniki odczytu. Obecne testy kontrolują te zachowania, ale nie egzekwują zakazu `saturating_*`.

### R-02 — medium — Test zerowania sekretu przepuszcza usunięcie zerowania

**Lokalizacja:** `crates/recipe-web/src/pages/machines/connection_request_tests.rs:99`.

Test `a_secret_field_clears_its_buffer_on_drop` sprawdza tylko:

```rust
assert!(std::mem::needs_drop::<SecretField>());
```

**Wymaganie:** `openspec/changes/archive/2026-11-11-plc-connection-config/design.md:65` wymaga zerowania `SecretField` przy zwalnianiu. `AGENTS.md:34` klasyfikuje słaby test, który przepuszcza wiarygodny błąd, jako medium.

**Błąd i warunki:** zastąpienie pola `Zeroizing<String>` zwykłym `String` oraz dostosowanie konstruktora nadal spełnia asercję. `String` wymaga destruktora także bez zerowania bufora. Taka mutacja usuwa wymaganą ochronę i nie jest wykrywana przez wskazaną asercję.

**Dowód i zabezpieczenia:** obecny kod jest prawidłowy. `connection_request.rs:29` przechowuje `Zeroizing<String>`, a konstruktor w linii 44 używa `Zeroizing::new`. Przeczytano pełny test i istotny kontekst produkcyjny. Wyszukiwanie nie znalazło innej kontroli strukturalnego kontraktu zerowania `SecretField`.

**Skutek:** fałszywe poczucie ochrony przed przyszłą regresją pozostawiającą hasło w zwalnianym buforze. Nie stwierdzono obecnego wycieku.

**Zakres poprawki:** zastąpić `needs_drop` testem kontraktu zerującego typu lub inną bezpieczną kontrolą powiązaną z jego implementacją. Test nie może odczytywać zwolnionej pamięci.

**Weryfikacja:** kontrolowana zamiana na zwykły `String` musi powodować błąd kompilacji lub testu. Mutacji ani testów Rust podczas przeglądu nie wykonywano. Ocena odporności obecnej asercji jest statyczna.

### R-03 — medium — ADR-0133 zawiera sprzeczne gwarancje liczby zawieszonych wątków

**Źródła konfliktu:**

- `docs/adr/0133-time-limit-of-the-disk-measurement.md:43` — decyzja (d) dopuszcza równoległe wywołania przed upływem limitu.
- `docs/adr/0133-time-limit-of-the-disk-measurement.md:53` — konsekwencje obiecują najwyżej jeden blokujący wątek na ścieżkę.

**Scenariusz:** `disk_check` i `alerts`, współdzielące adapter dysku, mogą rozpocząć odczyt tego samego `pgdata` przed oznaczeniem ścieżki. Jeżeli oba odczyty utkną, pozostaną dwa wątki.

**Dowód:** `crates/recipe-server/src/limited_read.rs:135` sprawdza oznaczenie przed uruchomieniem zadania w linii 140. Oznaczenie powstaje dopiero po timeout w linii 148. Test `crates/recipe-server/src/limited_read/tests.rs:262`, `two_marks_on_one_path_clear_one_by_one`, świadomie obsługuje kilka oznaczeń jednej ścieżki.

**Skutek:** dokumentacja zawyża gwarancję ograniczenia zasobów. Nie wykazano nieograniczonego wzrostu liczby wątków. Nie uznano implementacji za wadliwą wyłącznie dlatego, że odpowiada jednemu z dwóch sprzecznych zapisów.

**Wymagane rozstrzygnięcie:** czy wymagany jest jeden odczyt na ścieżkę, czy dopuszczona współbieżność przed timeout? Raport nie wybiera samodzielnie odpowiedzi.

**Zakres poprawki:** po decyzji właściciela dostosować mechanizm współdzielenia odczytu albo doprecyzować gwarancję ADR. Zachować procedurę zmian zaakceptowanych ADR z `docs/adr/README.md:214`.

**Weryfikacja:** dwa równoczesne zawieszone odczyty, odmowa kolejnego odczytu oraz niezależne kończenie wcześniejszych zadań. Oczekiwana liczba uruchomionych odczytów musi wynikać z rozstrzygniętej gwarancji.

### R-04 — medium — Przekroczone limity rozmiaru funkcji i plików

**Wymaganie:** `AGENTS.md:50`: funkcja ma najwyżej 50 wierszy, plik najwyżej 800. Wprost wskazany poziom naruszenia: medium.

**Funkcja:** `crates/recipe-app/src/plant/set_plc_connection.rs:111`, `SetPlcConnection::execute`, zajmuje 64 wiersze z sygnaturą i 57 wierszy treści. Kończy się w linii 174.

**Potwierdzone przekroczenia plików:**

| Plik | Pierwsza linia ponad limit | Liczba wierszy |
|---|---:|---:|
| `crates/recipe-app/src/plant/set_plc_connection/tests.rs` | 801 | 849 |
| `crates/recipe-db/tests/storage/constraints/plc_connection.rs` | 801 | 948 |
| `crates/recipe-db/tests/storage/plc_connection.rs` | 801 | 896 |
| `crates/recipe-domain/src/plant/connection/shape/tests.rs` | 801 | 877 |
| `crates/recipe-server/tests/deploy_values/file_mutations.rs` | 801 | 892 |
| `crates/recipe-server/tests/deploy_values/files.rs` | 801 | 907 |

**Skutek:** naruszenie wymagania utrzymywalności i trudniejsza kontrola rozbudowanych scenariuszy. Nie wykazano, że długość `execute` powoduje błąd atomowości.

**Zakres poprawki:** wydzielić spójne etapy komendy i grupy testów według scenariusza. Zachować jedną transakcję zapisu, kolejność blokad oraz audyt.

**Weryfikacja:** ponowne liczenie wierszy i istniejące testy konfliktu wersji, rollbacku, audytu oraz ochrony hasła. Nie jest potrzebny nowy test wyłącznie dla samego podziału pliku.

### R-05 — low — Nieaktualny kontrakt kolejności blokad typów pochodnych

**Lokalizacja:** `crates/recipe-app/src/ports/types.rs:111`.

Komentarz mówi, że wywołujący sortuje klucze, a adapter pobiera blokady w otrzymanej kolejności.

**Wymaganie:** `docs/adr/0144-order-of-the-class-2-locks-by-key-value.md:54` wymaga obliczenia wartości hashy, usunięcia duplikatów i sortowania tych wartości przez adapter. Linia 57 wyraźnie odbiera aplikacji odpowiedzialność za kolejność blokad.

**Dowód i zabezpieczenia:** implementacja PostgreSQL jest poprawna. `crates/recipe-db/src/transaction/type_rows.rs:46` definiuje `SELECT DISTINCT hashtext(...) ... ORDER BY key`.

**Skutek:** mylący kontrakt może skłonić przyszłą implementację do przywrócenia niewłaściwej kolejności. Nie wykazano obecnego deadlocku z tej przyczyny.

**Poprawka i weryfikacja:** uaktualnić komentarz względem ADR-0144 i rzeczywistego adaptera. Dodatkowy test wykonawczy dla samej korekty opisu nie jest potrzebny.

### R-06 — low — Runbook pomija eksport połączenia PLC w wyjątkach read-only

**Lokalizacja:** `docs/ops/runbook.md:2350`; zdanie w linii 2351 rozszerza odmowę na każdą pozostałą komendę.

Runbook wymienia zapytania, `Login`, `Logout` i `ExportVersion`, a następnie stwierdza, że każda inna komenda zwraca 503. Pomija `ExportPlcConnection`.

**Wymaganie:** `docs/adr/0154-plc-connection-settings.md:65` oraz `docs/architecture/api.md:430` dopuszczają `ExportPlcConnection` w read-only. `docs/architecture/overview.md:269` zawiera aktualne A24.

**Dowód i zabezpieczenia:** katalog `crates/recipe-app/src/auth/operation/catalog.rs:147` oznacza `ExportPlcConnection` jako komendę. Kod eksportu odpowiada aktualnemu wymaganiu i dopuszcza tę operację w read-only.

**Skutek:** operator może błędnie zinterpretować dostępność eksportu połączenia i związane z nim wpisy audytu podczas awarii dysku. Nie wykazano nieprawidłowego zachowania kodu.

**Poprawka i weryfikacja:** dopisać `ExportPlcConnection` albo użyć aktualnego sformułowania „każdy eksport”. Porównać opis z A24, API i katalogiem operacji.

## 3. Jednoznaczny przedmiot przeglądu

- Repozytorium: `D:/editor-web-cc`.
- HEAD: `4bd52e856d6e5f2abcb9be637307e904b9a7e900`.
- Gałąź: `main`; w chwili przeglądu `origin/main` wskazywał tę samą rewizję.
- Wszystkie pliki śledzone: 2362.
- Inwentaryzowany zakres: 2354 pliki z `crates/`, `migrations/`, `deploy/`, `docs/`, `openspec/`, `tools/`, `.github/`, `e2e/` i pliki w katalogu głównym.
- Jedyna lokalna zmiana śledzonego pliku: `.claude/settings.json`, dodanie `enabledPlugins` z `bash-scripting@cnative-skills: true`.
- Brak zmian staged oraz lokalnych zmian śledzonego kodu produktu.
- Nie było aktywnej zmiany OpenSpec poza archiwum. Przeczytano odpowiednie archiwalne projekty do interpretacji badanego kodu.
- Diff i historia zwiększyły priorytet konfiguracji połączeń PLC i ostatnich poprawek. Zakres `HEAD~5..HEAD` obejmował 277 plików, 23529 dodanych i 2031 usuniętych wierszy.

Nieśledzone elementy widoczne w statusie: `.agents/`, `.claude/agents/`, `.claude/hooks/context-watch.py`, `.claude/settings.local.json`, `.claude/skills/`, `.codex/`, `.serena/`, `skills-lock.json`, `support.rs`. Nie stanowiły ocenianego kodu produktu. `target/` i wskazane obszary narzędziowe były wyłączone.

Git zgłaszał brak dostępu do globalnego pliku ignore oraz nieśledzonych `.gstack/` i `tmp62t51ikk/`. Inwentaryzacja śledzonych plików i diff działały.

Odcisk SHA-256 zakresu na początku i końcu przeglądu:

```text
a1569a201ca539b12b014545004a2e775bca1be9c65bd25c6d719d5f5a70d959
```

Algorytm: w kolejności `git ls-files`, dla każdej ścieżki zakresu dodano do SHA-256 `pathUTF8 + NUL + SHA256(fileBytes).digest()`. Hash obejmuje lokalne bajty plików. Nie obejmuje później utworzonego, nieśledzonego pliku tego raportu.

Przed zapisaniem raportu ponownie sprawdzono HEAD i status. Odpowiadały stanowi badanemu w przeglądzie.

## 4. Kontekst architektoniczny i metoda

Przeczytano instrukcje AGENTS.md, CLAUDE.md, RTK oraz `.claude/rules/{rust,web,sql,docs,openspec}.md`. Wykorzystano `overview.md`, indeks i mapę ADR oraz dokumenty normatywne właściwe dla analizowanych przepływów.

Mapa komponentów:

1. Przeglądarka lub klient transferowy komunikuje się z Caddy przez TLS.
2. `recipe-web` odpowiada za HTTP, sesję, CSRF, formularze i odpowiedzi.
3. `recipe-app` wykonuje autoryzację, przypadki użycia i koordynację transakcji przez porty.
4. `recipe-domain` zawiera reguły bez I/O, w tym kanonizację i część eksportu objętą hashem.
5. `recipe-db` implementuje porty PostgreSQL, blokady, zapis i audyt.
6. `recipe-export` buduje zewnętrzną reprezentację eksportu.
7. `recipe-server` składa adaptery, ładuje konfigurację i sekrety oraz nadzoruje zadania i proces.

Granice zaufania obejmują klienta HTTP, proxy, proces aplikacji, role PostgreSQL, administratora bazy i hosta, pliki kluczy oraz magazyn kopii. Aplikacja przechowuje konfigurację PLC, ale nie komunikuje się z PLC. Transfer wykonuje zewnętrzny adapter. Braku sterownika PLC nie uznano za usterkę.

Kolejność analizy odpowiadała ryzyku: transakcje i audyt → tożsamość i sekrety → domena i eksport → HTTP → praca procesu i odtwarzanie → jakość testów i zgodność dokumentów.

Dla przepływów wysokiego ryzyka prześledzono wejście, uwierzytelnienie, uprawnienia, walidację, transakcję, blokady, zapis, audyt i odpowiedź. Sprawdzano także odmowy, konflikt równoległy, timeout, przerwanie i późny błąd zależności. Wyszukiwanie było wskazówką, a nie samodzielnym dowodem.

## 5. Pokrycie

| Obszar | Zakres i metoda | Stan |
|---|---|---|
| Domena, liczby, kanonizacja | Wszystkie 152 produkcyjne moduły domeny; zakresy, Current/Earlier, limity, rebind, S7 | Kod pełny; testy wybrane |
| Uprawnienia, tożsamość, sesje, sekrety | Pełny kod identity i autoryzacji; ponowne sprawdzenie aktora, epoch, bootstrap, Basic, PLC | Kod pełny; testy wybrane |
| Transakcje i współbieżność | Pełny kod aplikacji i DB; approval, edycje, archiwizacja, typy pochodne, PLC | Kod pełny; wszystkie 17 plików `storage/{atomic,race}` przeczytane |
| Audyt | Atomowość commit, łańcuch, kotwica, granty i rollback po błędzie audytu | Produkcyjne ścieżki pełne; testy wybrane |
| Baza i migracje | Wszystkie 50 produkcyjnych plików DB oraz 12 migracji SQL | Kod i migracje pełne; testy DB częściowo |
| HTTP i UI | Wszystkie 132 produkcyjne moduły web, 78 produkcyjnych szablonów, własny JS, CSS, cztery Fluent | Własny kod pełny; wnętrze vendor nieaudytowane |
| Eksport | Domena eksportu i wszystkie 5 plików `recipe-export`; v1/v2, połączenie PLC, hash i audyt | Kod pełny; sześć golden hashy niezależnie sprawdzonych |
| Read-only i proces | Wszystkie 37 produkcyjnych plików server; supervisor, watchdog, limity, drain i stop | Kod pełny; brak własnego wykonania scenariuszy |
| Wdrożenie i odtwarzanie | Compose, role, główne pgtools, trzy workflow, dodatkowy pełny przegląd PITR/restore-volume | Główne ścieżki pełne; pozostałe testy i runbook częściowo |
| Specyfikacje i E2E | Wszystkie 7 aktualnych `spec.md` oraz 26 plików E2E | Pełny odczyt; bez pełnej macierzy wszystkich scenariuszy do asercji |

Produkcja Rust: app 155, DB 50, domain 152, export 5, server 37, web 132 — łącznie **531 plików/modułów**. Mianownik ustalono z produkcyjnego grafu modułów, z wyłączeniem modułów testowych, a nie tylko z nazw plików.

Aktualne OpenSpec:

| Spec | Linie | Wymagania | Scenariusze |
|---|---:|---:|---:|
| audit | 464 | 34 | 62 |
| export | 712 | 55 | 95 |
| identity-access | 776 | 62 | 98 |
| operations | 2198 | 120 | 333 |
| plant-model | 1272 | 62 | 202 |
| recipe-management | 1621 | 92 | 248 |
| web-ui | 2667 | 161 | 384 |
| Razem | 9710 | 586 | 1422 |

Wszystkie 118 śledzonych szablonów objęto kontrolą strukturalną. Pełny odczyt semantyczny dotyczył 78 szablonów produkcyjnych. Pozostałe 40 to fixtures testowe.

### Dodatkowe pokrycie testów wysokiego ryzyka

W `crates/recipe-db/tests/storage/atomic/` przeczytano wszystkie 9 plików: `draft`, `export`, `identity`, `mod`, `plant`, `plc_connection`, `recipe`, `text_nul`, `world`.

W `crates/recipe-db/tests/storage/race/` przeczytano wszystkie 8 plików: `approve`, `derived_locks`, `identity`, `mod`, `order`, `plc_connection`, `refused_type`, `struct_name_case`.

To 4381 wierszy i 100 deklaracji testów. Przeczytano także helpery `support/{app,gate,fault,probe,hold,writers,identity,fixtures,audit}.rs` oraz `storage/settings.rs` i `storage/maintenance.rs`.

Bramki testów potwierdzają punkt synchronizacji przez `pg_stat_activity`, `pg_blocking_pids` lub `pg_locks`. Wstrzykiwane błędy sprawdzają konkretny SQLSTATE lub błąd audytu. Asercje obejmują stan biznesowy, `row_version`, edytorów draftu i nieprzesunięcie głowy audytu. Nie opierają sukcesu wyłącznie na czasie albo dowolnym `Err`.

Pełny dodatkowy przegląd pgtools objął `bin/pitr-volume`, `bin/restore-volume`, `lib/recovery-steps.sh`, `lib/restore-steps.sh`, `lib/wal-identity.sh` i `deploy/db/cluster-access.sh` — 1790 wierszy. Przeczytano też całe ADR-0084, 0095, 0103–0107 i 0151.

Pełny odczyt dodatkowych testów pgtools: 16 plików, 3694 wiersze. Są to `pitr-volume-test`, `restore-volume-test`, `pitr-start-test`, `lib/helpers.sh`, `lib/pitr-volume-stubs.sh`, wszystkie `lib/pitr-volume-*-cases.sh`, `lib/restore-volume-stubs.sh` oraz `lib/restore-volume-{cases,gate-cases,start-cases,stop-cases}.sh`. `wal-entry-cases.sh` odczytano w tym etapie w liniach 180–691.

Potwierdzono w kodzie odmowę użycia niepustego celu, prywatny socket, kontrolę system identifier i timeline, weryfikację kopii i audytu, brak publikacji sukcesu po błędzie oraz cleanup własnego procesu. Nie wykonano rzeczywistego odtwarzania.

## 6. Wykonane kontrole i ograniczenia narzędzi

| Kontrola | Zakres i wynik | Exit |
|---|---|---:|
| Git, diff, historia, inwentaryzacja i odcisk | Ustalony HEAD i lokalny stan; zgodność początku i końca | 0 |
| `ocr delegate preview --format json` | CLI `ocr` niedostępne | 1 |
| Parsowanie Python i TOML | 34 pliki Python i 7 manifestów Cargo; bezpośredni graf zależności zgodny | 0 |
| `python -I -B deploy/ci/migration_rewrite.py migrations` | 12 migracji, bez zgłoszeń | 0 |
| Bash `--noprofile --norc -n` | 78 skryptów; poprawna składnia bez wykonania | 0 dla każdego |
| STE lint i self-test | 186 dokumentów w zakresie CI; 0 naruszeń twardych, 336 uwag doradczych | 0 |
| `python -I -B tools/coverage-gate.py --source-only` | Brak naruszeń reguł kwalifikowania źródeł; nie jest to pomiar coverage | 0 |
| Rejestr błędów | 122 stałe = 122 wpisy rejestru = 122 wiersze dokumentacji; bez różnic | 0 |
| Szablony | 118 plików, bez wykrytych zabronionych konstrukcji | 0 |
| Fluent | 953 klucze na język; brak duplikatów i różnic nazw argumentów interpolacji | 0 |
| Golden eksportu | Sześć niezależnie przeliczonych hashy zgodnych | 0 |
| Schematy eksportu | Parsowanie i porównanie strukturalne v1/v2; pełny lokalny walidator JSON Schema niedostępny | 0 dla wykonanej części |
| Zasoby statyczne | HTMX 2.0.11 i osiem WOFF2 zgodne z dziewięcioma pinami SHA-256 w repo | 0 |

Zgodność pinów nie dowodzi pochodzenia upstream ani bezpieczeństwa wewnętrznego kodu vendor. Niezależne przeliczenie golden hashy dotyczy konkretnych fixture'ów i ich podzbioru kanonizacji, nie dowodzi całego RFC 8785.

Pomocnicze błędy kodowania, globów i cytowania miały exit 1. Poprawione odczyty zakończyły się 0. Wyniki ucięte przez limit wyjścia odczytywano ponownie przed uznaniem pełnego pokrycia. Ostrzeżenie Pythona o sekwencji `\$` w jednym pliku testowym nie spowodowało niepowodzenia parsowania.

Użyto zainstalowanego skill `open-code-review-codex:open-code-review-delegate` z `C:/Users/rtm/.codex/plugins/cache/open-code-review/open-code-review-codex/1.0.0/skills/open-code-review-delegate/SKILL.md`. Brak CLI uniemożliwił automatyczny wybór plików i rozstrzygnięcie reguł przez OCR. Dobór wykonano ręcznie z Git i dokumentów repozytorium. Liczniki OCR są niedostępne, a nie równe zero. Nie używano osobnego endpointu LLM i nie instalowano zależności.

Pomocnicze skills obejmowały przegląd Rust, współbieżność, model PostgreSQL, Askama i STE. Równoległe przebiegi były częścią instrukcji użytego skill przeglądu Rust. Nie publikowano komentarzy w usługach zewnętrznych.

**Niewykonane lokalnie:** wszystkie polecenia cargo, kompilacja, testy Rust, Clippy, audyt zależności wymagający cargo, kontenery, browser tests, load/soak, fault injection, mutation testing i odtwarzanie. Zakaz cargo pochodził od użytkownika i AGENTS.md. Statycznego odczytu testów nie traktowano jako ich przejścia.

## 7. Dowody CI dla dokładnego HEAD

Oba runy dotyczą `4bd52e856d6e5f2abcb9be637307e904b9a7e900`, zdarzenia push na `main`, z początkiem 2026-11-11 11:58:18 UTC:

- [CI — 43016323332](https://github.com/rtmachines/editor-web-cc/actions/runs/43016323332): `completed`, `success`.
- [Browser regression — 43016323329](https://github.com/rtmachines/editor-web-cc/actions/runs/43016323329): `completed`, `success`.

W CI wszystkie 18 jobów miało `success`: Upgrade an older volume, Clippy, Load (cc-debian13), Panic unwind on musl, Load (musl), Recover from the WAL archive, Migration rewrite, Dependency policy, Restore into a new volume, CI scripts, Compose stack, Format, Coverage, STE lint, Documentation, Test, OpenSpec specs, Migration freeze.

Pobrany log joba Test zawiera 35 podsumowań: 8927 wyników passed, 0 failed, 0 ignored. Suma obejmuje oddzielne uruchomienia tego joba, w tym dodatkową kontrolę secure-cookie. Nie jest przedstawiana jako liczba unikalnych scenariuszy biznesowych.

Pobrany log bramki coverage:

| Crate | Pokryte linie | Linie | Coverage |
|---|---:|---:|---:|
| recipe-app | 7859 | 8081 | 97,25% |
| recipe-db | 3386 | 3553 | 95,30% |
| recipe-domain | 9249 | 9353 | 98,89% |
| recipe-export | 142 | 143 | 99,30% |
| recipe-server | 2874 | 2982 | 96,38% |
| recipe-web | 13287 | 13556 | 98,02% |

Status success pochodzi z GitHub Actions i nie jest lokalnym kodem zakończenia. Sukces joba nie dowodzi spełnienia każdej metryki informacyjnej lub opcjonalnej. Nie przypisano wyników innej rewizji bieżącemu kodowi. Nie potwierdzono osobnego nightly dependency audit dla tego HEAD.

## 8. Luki dowodowe, pytania i zaakceptowane ograniczenia

### 8.1. Praca 24/7 nie jest potwierdzona testem długotrwałym

`docs/architecture/reliability.md:527` wymaga 24-godzinnego soak, cyklicznych fal logowania, braku restartów i błędów, pamięci poniżej 384 MiB oraz p99 poniżej 500 ms.

`docs/ops/runbook.md:2500` potwierdza brak implementacji i wykonania tego poziomu testów, w tym brak wcześniejszego 2-godzinnego lokalnego przebiegu z wykresem RSS. `openspec/changes/archive/2026-11-09-test-gates-round-2/proposal.md:48` jawnie pozostawia soak jako znaną lukę poza zakresem tej zmiany.

To jawnie odłożona praca, a nie nowa wykazana regresja. Zielone CI nie zamyka tej luki. Przed uznaniem instalacji za zweryfikowaną do pracy 24/7 należy uzyskać wynik tego testu dla właściwego obrazu i konfiguracji.

### 8.2. Wolne miejsce podczas recovery drill — hipoteza wymagająca próby

`docs/adr/0106-recovery-of-production-from-the-wal-archive.md:100` nakazuje preflight, a linia 101 dodatkowo deklaruje zachowanie progu wolnego miejsca po kopii i replay. `docs/ops/runbook.md:1000` stwierdza, że drill nie wprowadzi aplikacji w read-only.

`deploy/pgtools/bin/pitr-volume:390` sprawdza jednorazowo sumę rozmiaru kopii i plików segmentów. Test `deploy/pgtools/tests/lib/pitr-volume-select-cases.sh:246` sprawdza granicę prognozy, błędne `df` i uwzględnienie segmentów przez stuby.

Nie wykazano naruszenia tej gwarancji podczas rzeczywistego replay ani błędu w rachunku preflight. Nie przypisano wagi findingu. Potrzebna jest próba z pomiarem szczytowego zajęcia dysku na reprezentatywnej kopii i WAL, także przy równoległej pracy aplikacji, albo osobny argument o górnej granicy zużycia.

### 8.3. Layout PLC wymaga dalszego dowodu fizycznego

`docs/adr/0147-s7-layout-from-the-siemens-rules.md:58` i linia 59 akceptują tymczasową interpretację trójwymiarowej tablicy BOOL. Linia 61 pozostawia A03 jako Unverified, wymagające pomiaru TIA Portal. Kod odpowiada tej decyzji. Nie jest to nowa potwierdzona usterka. Pomiar jest nadal potrzebny do zamknięcia ograniczenia dowodów.

### 8.4. Zaufanie podczas odtwarzania

Ryzyko wykonania kodu spreparowanego dumpa przed pełną kontrolą oraz ograniczona wykrywalność zmian katalogu są jawnie zaakceptowane w ADR-0084, ADR-0095 i ADR-0103. `docs/ops/runbook.md:2497` opisuje ryzyko dla klastra, który potem staje się produkcyjny.

Nie zgłoszono tego ponownie jako nowego exploitu. Przegląd nie dowodzi odporności na przejęcie administratora hosta lub superusera bazy, w tym przeliczenie całego łańcucha audytu i kotwicy. Istotne pozostają zabezpieczenia kopii poza zakładem i niezależnej historii.

### 8.5. Dowody środowiskowe i ograniczenia operacyjne

- Kopie poza zakładem, ich harmonogram, narzędzie, weryfikacja base backup/WAL oraz odtworzenie sekretów wymagają dowodu z konkretnej instalacji. Runbook jawnie wymienia część tych braków w sekcji Known gaps.
- Klucz PLC musi być dostępny po awarii. Brak rotacji tego klucza jest świadomym ograniczeniem ADR-0154, a nie nową usterką.
- ADR-0150 jawnie akceptuje brak limitu per login ponad wszystkimi adresami IP i brak security event dla błędnego aktualnego hasła w ChangeOwnPassword. Nie zgłoszono ich jako nowych błędów.
- Samo line coverage, nawet powyżej 95%, nie dowodzi skuteczności asercji ani pokrycia wszystkich 1422 scenariuszy specyfikacji.
- Wnętrze HTMX, fontów i pełny transytywny łańcuch zależności nie zostały niezależnie zaudytowane. Zgodność z pinami repozytorium nie dowodzi pochodzenia upstream.

## 9. Istotne hipotezy odrzucone po sprawdzeniu kontekstu

1. **Niespójny snapshot zatwierdzenia.** Adapter odczytuje nagłówek z row_version przed dziećmi. Etap zapisujący ponownie porównuje wersje pod blokadą. Nie potwierdzono błędu tokenu precheck.
2. **Powtarzalny wyciek spawn_blocking po porzuceniu read_with_limit.** Prześledzono trzech produkcyjnych wywołujących: disk, check_results i backups. Bieżące cykle czekają na zakończenie; znalezione anulowanie wiąże się z zamykaniem lub przerwaniem startu. Nie potwierdzono osiągalnej powtarzalnej awarii produkcyjnej.
3. **Dwie transakcje Login i ChangeOwnPassword.** To jawny wyjątek ADR-0035(b). Druga transakcja ponownie sprawdza stan po haszowaniu. Nie zgłoszono naruszenia atomowości tylko na podstawie liczby transakcji.
4. **Historia dla każdej roli.** Ogólny zapis `identity-access/spec.md:186` doprecyzowuje późniejszy przepis ExportClient i ADR-0109(b). Konto techniczne nie ma RecipeView. Nie ma nierozstrzygniętego konfliktu.
5. **Wyłączenie timeoutu tylko dla v1.** ADR-0109(q) jawnie rozszerza ADR-0072 na całe `/export/`. Kod korzysta z tego prefiksu. Istnieje test slow v2 w `crates/recipe-web/src/error/export_json_tests.rs:520`. Odczytano go statycznie.
6. **Słabość browser testu wielowierszowego tekstu.** Identyczna wartość skalarna w seedzie i stałej testu nie wystarcza do findingu. Test sprawdza dokładne LF po zapisie i reload, a test Rust sprawdza CRLF→LF i odmowę samotnego CR.
7. **Brak asercji rotacji sesji w komentarzu Compose login.** Komentarz sam nie dowodzi rotacji, ale `crates/recipe-web/src/pages/own/tests.rs:577` porównuje cookie, odrzuca stare, akceptuje nowe i sprawdza ponowne logowanie.

## 10. Kolejność napraw i pozostałego przeglądu

1. Usunąć naruszenia R-01 i wzmocnić test ochrony sekretu R-02.
2. Rozstrzygnąć gwarancję ADR-0133 z R-03 przed wyborem poprawki mechanizmu lub dokumentu.
3. Przed uznaniem instalacji za zweryfikowaną do pracy 24/7 wykonać soak i recovery drill na reprezentatywnych danych, wraz z odtworzeniem sekretów.
4. Podzielić wskazane funkcje i pliki z R-04 oraz poprawić opisy R-05 i R-06.
5. Zbudować pełne powiązanie 1422 scenariuszy aktualnych specs z konkretnymi asercjami. Zweryfikować luki i odporność na wiarygodne błędy.
6. Domknąć pozostałe testy domeny, aplikacji, DB i web. W DB nadal częściowe są rodziny contract, constraints, triggers, grants i część testów migracji.
7. Domknąć pozostałe harnessy pgtools i CI. W dodatkowym zadaniu PITR poza pełnym odczytem pozostały m.in. `restore-volume-access-cases.sh`, ogólne `restore-stubs.sh`, `restore-helpers.sh`, początek `wal-entry-cases.sh` i pozostałe biblioteki testowe.
8. Domknąć pełne procedury runbooka i dokumenty historyczne istotne dla niepokrytych scenariuszy. Aktualne siedem specs przeczytano w całości, lecz nie cały korpus archiwalnych zmian.
9. Osobno zweryfikować pochodzenie i bezpieczeństwo zależności i artefaktów vendor.

Brak wykazanego critical nie jest dowodem bezpieczeństwa całej aplikacji. Istniejące zabezpieczenia i wyniki CI stanowią istotne dowody, ale nie zamykają wskazanego pozostałego zakresu ani prób środowiskowych.
