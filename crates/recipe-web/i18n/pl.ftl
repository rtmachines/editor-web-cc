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
field-query = Pola wyszukiwania
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
