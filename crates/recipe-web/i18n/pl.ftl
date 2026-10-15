# Teksty edytora receptur w języku polskim (api.md, sekcja 8).
# Zadanie 6.4 uzupełnia ten plik. Każdy klucz z en.ftl ma tu odpowiednik.

language-name = Polski

# Zdanie pod każdym kodem błędu. Zawiera identyfikator żądania.
error-reference = Numer referencyjny: { $request_id }.

error-E-IDN-003 = Login lub hasło jest niepoprawne.
error-E-IDN-004 = Login lub hasło jest niepoprawne.
error-E-IDN-005 = Zbyt wiele nieudanych prób logowania z tego adresu. Spróbuj ponownie za { $retry_after } s.
error-E-WEB-002 = Twoja rola nie pozwala na operację { $operation }.
error-E-WEB-003 = Żądanie jest większe niż { $limit }.
error-E-WEB-004 = Zaloguj się, aby kontynuować.
error-E-WEB-007 = Formularz jest już nieważny. Odśwież stronę i spróbuj ponownie.
error-E-WEB-008 = Pole { $field } jest niepoprawne: { $reason }.
error-E-IDN-001 = Login { $login } jest już zajęty.
error-E-IDN-002 = { $login } jest ostatnim aktywnym administratorem. Najpierw dodaj innego administratora.
error-E-IDN-006 = Hasło nie zostało przyjęte: { $reason }. Użyj od 12 do 128 znaków, których nie ma na liście haseł z wycieków.
error-E-IDN-008 = Obecne hasło jest niepoprawne.
error-E-IDN-009 = Użytkownik { $login } jest już aktywny.
error-E-IDN-010 = Użytkownik { $login } jest już nieaktywny.
error-E-WEB-005 = Zasób nie istnieje: { $entity }.
error-E-WEB-006 = System działa w trybie tylko do odczytu, bo brakuje miejsca na dysku. Skontaktuj się z administratorem.
error-E-WEB-009 = Żądanie trwało zbyt długo i zostało przerwane. Spróbuj ponownie.
error-E-WEB-010 = Wystąpił błąd wewnętrzny. Skontaktuj się z administratorem.
error-E-WEB-011 = Operacja { $stage } nie zakończyła się. Spróbuj ponownie.
error-E-WEB-001 = Inny użytkownik zmienił wersję { $number } receptury { $reference }. Odśwież stronę i zastosuj zmianę ponownie.
error-E-DB-001 = Inna operacja blokuje te same dane. Spróbuj ponownie.
error-E-DB-002 = Operacja trwała zbyt długo i została przerwana. Spróbuj ponownie.
error-E-DB-003 = Zmiana narusza regułę bazy danych.
error-E-DB-004 = Brak wolnego połączenia z bazą danych. Spróbuj ponownie.
error-E-DB-005 = Baza danych nie odpowiada. Spróbuj ponownie.
error-E-DB-006 = Zmiana koliduje z istniejącym wpisem.
error-E-DB-007 = Zmiana odwołuje się do wpisu, którego nie ma.
error-E-DB-008 = Wystąpił nieoczekiwany błąd bazy danych. Skontaktuj się z administratorem.
error-E-AUD-001 = Zmiana nie została zapisana. System nie mógł zapisać wpisu audytu. Skontaktuj się z administratorem.

# Role
role-administrator = Administrator
role-process-engineer = Inżynier procesu
role-operator = Operator

# Strona logowania
login-title = Logowanie
login-label-login = Login
login-label-password = Hasło
login-submit = Zaloguj

# Własne konto
me-title = Moje konto
me-label-login = Login
me-label-role = Rola
me-link-password = Zmień hasło
logout-submit = Wyloguj

# Strona zmiany hasła
password-title = Zmiana hasła
password-label-current = Obecne hasło
password-label-new = Nowe hasło
password-hint = Użyj od 12 do 128 znaków. Nie używaj hasła z listy haseł z wycieków.
password-must-change = Musisz zmienić hasło jednorazowe, zanim otworzysz inne strony.
password-submit = Zmień hasło
password-changed = Hasło zostało zmienione.

# Lista użytkowników
users-title = Użytkownicy
users-search-label = Login zawiera
users-search-submit = Szukaj
users-new-link = Nowy użytkownik
users-empty = Nie znaleziono żadnego użytkownika.
users-col-login = Login
users-col-role = Rola
users-col-status = Stan
users-col-password = Zmiana hasła
users-must-change-yes = Wymagana
users-prev = Poprzednia strona
users-next = Następna strona
user-status-active = Aktywny
user-status-inactive = Nieaktywny

# Strona nowego użytkownika
user-new-title = Nowy użytkownik
user-new-label-login = Login
user-new-label-role = Rola
user-new-hint = Login ma od 3 do 64 znaków: litery od a do z, cyfry od 0 do 9, kropkę, podkreślenie i myślnik.
user-new-submit = Utwórz użytkownika
user-created-title = Użytkownik utworzony
user-created = Użytkownik { $login } został utworzony.

# Strona użytkownika
user-title = Użytkownik
user-label-login = Login
user-label-role = Rola
user-label-status = Stan
user-label-password = Zmiana hasła
user-label-created = Utworzony
user-label-deactivated = Dezaktywowany
user-label-failures = Nieudane logowania w oknie czasu
user-label-last-failure = Ostatnie nieudane logowanie
user-never = Brak
user-role-label = Nowa rola
user-role-submit = Ustaw rolę
user-deactivate-submit = Dezaktywuj
user-activate-submit = Aktywuj
user-reset-submit = Zresetuj hasło
user-unlock-submit = Odblokuj logowanie
user-back-link = Wróć do użytkownika

# Wynik polecenia
user-password-label = Hasło jednorazowe
user-password-once = Zapisz hasło teraz. System pokazuje je tylko raz. Użytkownik musi zmienić je przy pierwszym logowaniu.
user-role-changed = Rola użytkownika { $login } to teraz { $role }. Wszystkie sesje użytkownika zakończyły się.
user-deactivated = Użytkownik { $login } został dezaktywowany. Wszystkie sesje użytkownika zakończyły się.
user-activated = Użytkownik { $login } jest aktywny.
user-password-reset = Hasło użytkownika { $login } zostało zresetowane. Wszystkie sesje użytkownika zakończyły się.
user-unlocked = Opóźnienie logowania użytkownika { $login } zostało zdjęte.

# Layout
app-name = Edytor receptur
skip-to-content = Przejdź do treści
nav-label = Główna
nav-recipes = Receptury
nav-users = Użytkownicy
nav-me = Moje konto
lang-switch-label = Język
lang-switch-pl = Polski
lang-switch-en = English
user-signed-in = Zalogowano jako

# Recipe list
recipes-title = Receptury
recipes-empty = Brak receptur do pokazania.

# States
status-draft = Szkic
status-approved = Zatwierdzona
status-retired = Wycofana

# Forms
form-required = wymagane

# Error page
error-page-title = Żądanie nie powiodło się
error-page-home = Otwórz listę receptur

# Field labels
field-form = Formularz
field-lang = Język
field-login = Login
field-password = Hasło
field-current-password = Obecne hasło
field-role = Rola
field-query = Wyszukiwanie
field-q = Login zawiera
field-page = Strona
field-X-Forwarded-For = Nagłówek X-Forwarded-For

# Field reasons
reason-form = w formularzu brakuje pola, jest nieznane pole albo zły typ treści
reason-lang = język musi być pl albo en
reason-login-length = login ma od 1 do 64 znaków
reason-login-control = login nie zawiera znaku sterującego
reason-login-grammar = login ma od 3 do 64 znaków: litery a–z, cyfry 0–9, kropka, podkreślnik i łącznik
reason-password-length = hasło ma od 1 do 128 znaków
reason-role = wybierz rolę z listy
reason-query = adres ma nieznany parametr albo parametr, który nie jest tekstem
reason-q = tekst wyszukiwania ma najwyżej 64 znaki: litery a–z, cyfry 0–9, kropka, podkreślnik i łącznik
reason-page = numer strony to liczba całkowita od 1
reason-forwarded = każdy wpis musi być adresem IP

# Missing entities
missing-machine = Maszyna { $code }
missing-machine-id = Maszyna { $id }
missing-unit = Jednostka { $path } maszyny { $code }
missing-type = Typ { $name } maszyny { $code }
missing-recipe = Receptura { $reference }
missing-version = Wersja { $number } receptury { $reference }
missing-user = Użytkownik { $login }
missing-malformed-login = Użytkownik

# Password reasons and task stages
password-reason-empty = jest puste
password-reason-too-short = ma mniej niż 12 znaków
password-reason-too-long = ma więcej niż 128 znaków
password-reason-breached = jest na liście haseł, które wyciekły
task-stage-password-hash = obliczenie skrótu hasła
task-stage-validation = walidacja
task-stage-export = eksport
task-stage-token = generowanie tokenu

# Operations
operation-create-machine = Utworzenie maszyny
operation-rename-machine = Zmiana nazwy maszyny
operation-archive-machine = Archiwizacja maszyny
operation-add-unit = Dodanie jednostki
operation-rename-unit = Zmiana nazwy jednostki
operation-archive-unit = Archiwizacja jednostki
operation-define-udt = Zdefiniowanie typu UDT
operation-create-recipe = Utworzenie receptury
operation-rename-recipe = Zmiana nazwy receptury
operation-delete-recipe = Usunięcie receptury
operation-restore-recipe = Przywrócenie receptury
operation-create-version = Utworzenie wersji
operation-discard-draft = Odrzucenie szkicu
operation-approve-version = Zatwierdzenie wersji
operation-retire-version = Wycofanie wersji
operation-add-parameter = Dodanie parametru
operation-update-parameter-definition = Zmiana definicji parametru
operation-set-parameter-value = Ustawienie wartości parametru
operation-replace-limits = Zastąpienie limitów
operation-upsert-translation = Zapisanie tłumaczenia
operation-remove-translation = Usunięcie tłumaczenia
operation-remove-parameter = Usunięcie parametru
operation-rebind-type-revision = Przepięcie rewizji typu
operation-login = Logowanie
operation-logout = Wylogowanie
operation-create-user = Utworzenie użytkownika
operation-set-user-role = Ustawienie roli użytkownika
operation-deactivate-user = Dezaktywacja użytkownika
operation-activate-user = Aktywacja użytkownika
operation-reset-password = Reset hasła
operation-change-own-password = Zmiana własnego hasła
operation-unlock-login = Odblokowanie logowania
operation-export-version = Eksport wersji
operation-list-machines = Lista maszyn
operation-get-machine = Podgląd maszyny
operation-get-unit-tree = Podgląd drzewa jednostek
operation-get-unit = Podgląd jednostki
operation-list-types = Lista typów
operation-get-type-tree = Podgląd drzewa typu
operation-get-type-usage = Podgląd użycia typu
operation-list-recipes = Lista receptur
operation-get-recipe = Podgląd receptury
operation-list-versions = Lista wersji
operation-get-version = Podgląd wersji
operation-diff-versions = Porównanie wersji
operation-get-recipe-history = Podgląd historii receptury
operation-list-parameters = Lista parametrów
operation-get-parameter = Podgląd parametru
operation-get-limits = Podgląd limitów
operation-get-translations = Podgląd tłumaczeń
operation-list-users = Lista użytkowników
operation-get-user = Podgląd użytkownika
operation-list-audit-entries = Lista wpisów audytu
operation-get-audit-entry = Podgląd wpisu audytu
operation-list-security-events = Lista zdarzeń bezpieczeństwa
operation-get-own-profile = Podgląd własnego profilu
operation-get-machine-form = Formularz maszyny
operation-get-unit-form = Formularz jednostki
operation-get-udt-form = Formularz typu UDT
operation-get-recipe-form = Formularz receptury
operation-get-version-form = Formularz wersji
operation-get-parameter-form = Formularz parametru
operation-get-approval-preview = Podgląd przed zatwierdzeniem
operation-get-user-form = Formularz użytkownika
operation-get-login-form = Formularz logowania
operation-get-own-password-form = Formularz zmiany hasła

# Plant model pages (design D13 of web-plant-model)
nav-machines = Maszyny
status-archived = Zarchiwizowana
field-code = Kod maszyny
field-machine-name = Nazwa maszyny
field-plc-platform = Platforma PLC
field-main-unit-name = Nazwa jednostki głównej
field-parent-path = Jednostka nadrzędna
field-segment = Ostatni numer ścieżki jednostki
field-unit-name = Nazwa jednostki
field-type-name = Nazwa typu
field-members = Składniki
field-machine-q = Kod lub nazwa zawiera
field-show-archived = Pokaż zarchiwizowane maszyny
field-archived = Zarchiwizowany obiekt
field-add = Dodaj składnik
reason-machine-code = kod maszyny to M i liczba od 1 do 9999999
reason-name = nazwa ma od 1 do 200 znaków i nie ma znaków sterujących
reason-platform = wybierz platformę PLC z listy
reason-type-name = nazwa typu ma od 1 do 64 liter, cyfr i podkreśleń i zaczyna się literą lub podkreśleniem. Nie może to być new ani nazwa typu prostego
reason-member-name = nazwa składnika ma od 1 do 64 liter, cyfr i podkreśleń i zaczyna się literą lub podkreśleniem
reason-member-rows = każdy wiersz składnika wymaga nazwy i typu. Wyczyść oba pola wiersza, którego nie chcesz
reason-member-count = formularz ma najwyżej 1024 wiersze składników
reason-flag = wartość musi wynosić 1. Odśwież stronę i spróbuj ponownie
reason-search-text = szukany tekst ma najwyżej 200 znaków i nie ma znaków sterujących
missing-malformed-machine = Maszyna
missing-malformed-type = Typ
missing-type-revision = Rewizja { $revision } typu { $name } maszyny { $code }
platform-codesys-iec = CODESYS (IEC 61131-3)
platform-twincat3 = TwinCAT 3
platform-s7-1200 = Siemens S7-1200
platform-s7-1500 = Siemens S7-1500
platform-s7-300 = Siemens S7-300
column-code = Kod
column-name = Nazwa
column-platform = Platforma PLC
column-state = Stan
column-revision = Rewizja
column-members = Składniki
column-recipe = Receptura
column-version = Wersja
column-tag = Tag
column-type = Typ
pager-label = Strony
pager-previous = Poprzednia strona
pager-next = Następna strona
pager-current = Strona { $page }
machines-title = Maszyny
machines-caption = Maszyny linii
machines-empty = Żadna maszyna nie pasuje. Zarchiwizowane maszyny widać tylko z filtrem.
machines-search-submit = Szukaj
machines-new-link = Nowa maszyna
machine-archived-toast = Maszyna { $code } została zarchiwizowana.
machine-new-title = Nowa maszyna
machine-code-hint = M i liczba od 1 do 9999999, na przykład M7.
machine-platform-hint = Platformy nie można zmienić po pierwszej recepturze.
machine-main-unit-hint = System tworzy jednostkę główną U1 o tej nazwie.
machine-create-submit = Utwórz maszynę
machine-title = Maszyna { $code }
machine-types-link = Otwórz typy UDT
machine-units-title = Drzewo jednostek
machine-rename-submit = Zmień nazwę maszyny
machine-renamed = Maszyna { $code } ma nową nazwę.
machine-archive-summary = Archiwizuj maszynę
machine-archive-warning = Maszyna i jej drzewo jednostek będą tylko do odczytu. Tej operacji nie można cofnąć. Receptury pozostaną dostępne, a nowa maszyna może użyć kodu { $code }.
machine-archive-submit = Archiwizuj maszynę { $code }
machine-archived-notice = Ta maszyna jest zarchiwizowana. Strona jest tylko do odczytu.
machine-archived-at = Data archiwizacji
tree-add-main-unit = Dodaj jednostkę główną
tree-add-sub-unit = Dodaj podjednostkę
tree-add-sub-unit-to = Dodaj podjednostkę do { $path }
unit-tree-title = Drzewo jednostek maszyny { $code }
unit-new-title = Nowa jednostka maszyny { $code }
unit-new-under = Nowa jednostka w { $path }
unit-new-main = Nowa jednostka główna
unit-parent-none = Brak. Jednostka jest jednostką główną.
unit-segment-hint = Liczba od 1 do 64. Formularz proponuje pierwszy wolny numer.
unit-no-free-segment = Wszystkie numery od 1 do 64 w tej jednostce są zajęte.
unit-create-submit = Dodaj jednostkę
unit-added = Jednostka { $path } została dodana.
unit-archived-toast = Jednostka { $path } i jej podjednostki zostały zarchiwizowane.
unit-title = Jednostka { $path }
unit-machine = Maszyna
unit-path = Ścieżka jednostki
unit-children-title = Podjednostki
unit-children-empty = Jednostka nie ma podjednostek.
unit-rename-submit = Zmień nazwę jednostki
unit-renamed = Jednostka { $path } ma nową nazwę.
unit-archive-summary = Archiwizuj jednostkę
unit-archive-warning = Jednostka i wszystkie jej podjednostki zostaną zarchiwizowane. Tej operacji nie można cofnąć. System zachowa ich historię.
unit-archive-count = Podjednostki, które system także zarchiwizuje: { $count }.
unit-archive-submit = Archiwizuj jednostkę { $path }
types-title = Typy UDT maszyny { $code }
types-caption = Rewizje struktur
types-empty = Maszyna nie ma typów UDT.
types-new-link = Nowy typ UDT

# Review fixes of step P3.2 (design D13, D20)
error-E-PLT-001 = Kod maszyny { $code } jest już używany. Użyj innego kodu.
error-E-PLT-002 = Ścieżka jednostki { $path } ma więcej niż 5 poziomów. Użyj najwyżej 5 poziomów.
error-E-PLT-003 = Jednostka nadrzędna dla { $path } nie istnieje. Najpierw utwórz jednostkę nadrzędną.
error-E-PLT-004 = Jednostka { $path } już istnieje na tej maszynie.
error-E-PLT-005 = Typ { $type_name } zawiera sam siebie. Usuń składnik, który tworzy cykl.
error-E-PLT-006 = Składnik { $member } typu { $type_name } używa typu, który nie istnieje na platformie { $platform }.
error-E-PLT-008 = Jednostka { $path } jest ostatnią jednostką główną maszyny { $code }. Maszyna potrzebuje jednej jednostki głównej.
error-E-PLT-009 = Jednostka { $path } jest zarchiwizowana. Wybierz aktywną jednostkę.
error-E-PLT-010 = Maszyna { $code } jest zarchiwizowana.
error-E-PLT-011 = Maszyna { $code } ma wersję roboczą w recepturze { $reference }. Najpierw zatwierdź albo odrzuć wersję roboczą.
error-E-PLT-012 = Jednostka { $path } jest używana w wersji roboczej receptury { $reference }. Najpierw przenieś albo usuń te parametry.
error-E-PLT-013 = Typ { $type_expr } jest niepoprawny: { $reason }. Użyj typu takiego jak INT, STRING(20), ST_Zone@3 lub ARRAY[1..8] OF REAL.
error-E-PLT-014 = Jednostka { $path } jest używana w zatwierdzonej wersji receptury { $reference }. Administrator może wycofać tę wersję.
error-E-PLT-015 = Typ { $type_name } już istnieje na tej maszynie. Zamiast tego utwórz nową rewizję.
error-E-PLT-016 = Typ { $type_name } nie ma składników. Dodaj co najmniej jeden składnik.
error-E-PLT-017 = { $text } nie jest ścieżką jednostki. Ścieżka jednostki to U i od 1 do 5 liczb od 1 do 64, z kropką między dwiema liczbami. Przykład: U1.2.
error-E-PLT-018 = Typ { $type_name } przekracza limit { $limit }. Użyj mniej elementów, mniej składników lub mniej poziomów.
error-E-PLT-021 = Typ { $type_name } ma dwa składniki o nazwie { $member }. Nadaj każdemu składnikowi inną nazwę.
type-reason-grammar = tekst nie pasuje do gramatyki typu
type-reason-unknown-struct = struktura albo rewizja nie istnieje na tej maszynie
type-reason-not-on-platform = typ nie istnieje na platformie maszyny
type-reason-out-of-range = granica, wymiar albo długość jest poza zakresem
type-reason-row-shape = zapisany wiersz typu nie pasuje do swojego rodzaju
type-limit-leaves = { $count } elementów skalarnych
type-limit-depth = { $count } poziomów
type-limit-members = { $count } składników
field-platform-choose = Wybierz platformę PLC
list-page-after-end = Strona { $page } jest za ostatnią stroną.
list-first-page = Przejdź do pierwszej strony

type-latest = najnowsza
udt-new-title = Nowy typ UDT
udt-revision-title = Nowa rewizja { $name }
udt-name-hint = Najpierw litera lub podkreślenie, potem litery, cyfry i podkreślenia. Najwyżej 64 znaki.
udt-member-row = Składnik { $number }
udt-member-name = Nazwa składnika { $number }
udt-member-type = Typ składnika { $number }
udt-type-hint = Wyrażenie typu, na przykład INT, STRING(20), ST_Zone@3 lub ARRAY[1..8] OF REAL.
udt-empty-rows-hint = System pomija wiersz z pustą nazwą i pustym typem. Wyczyść oba pola składnika, którego nie chcesz.
udt-add-member = Dodaj składnik
udt-max-rows = Formularz ma maksymalną liczbę 1024 składników.
udt-create-submit = Utwórz typ
udt-revision-submit = Utwórz rewizję
type-title = Typ { $name }@{ $revision }
type-revisions-title = Rewizje
type-tree-title = Drzewo typu
type-supersedes = Zastępuje
type-usage-link = Pokaż użycie tej rewizji
type-new-revision-link = Nowa rewizja
usage-title = Użycie { $name }@{ $revision }
usage-caption = Parametry, które używają rewizji
usage-empty = Żaden parametr nie używa tej rewizji.
