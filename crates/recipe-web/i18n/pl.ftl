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
