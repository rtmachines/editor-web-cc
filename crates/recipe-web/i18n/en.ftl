# Texts of the recipe editor in English (api.md section 8).
# Task 6.4 completes this file. Every key here has a key in pl.ftl.

language-name = English

# The sentence under each error code. It holds the request id.
error-reference = Reference: { $request_id }.

error-E-IDN-003 = Login or password is not correct.
error-E-IDN-004 = Login or password is not correct.
error-E-IDN-005 = Too many failed logins from this address. Try again in { $retry_after } seconds.
error-E-WEB-002 = Your role does not permit the operation { $operation }.
error-E-WEB-003 = The request is larger than { $limit }.
error-E-WEB-004 = Log in to continue.
error-E-WEB-007 = The form is no longer valid. Reload the page and try again.
error-E-WEB-008 = Field { $field } is not valid: { $reason }.
error-E-IDN-001 = Login { $login } is already in use.
error-E-IDN-002 = { $login } is the last active administrator. Add another administrator first.
error-E-IDN-006 = The password is not accepted: { $reason }. Use 12 to 128 characters that are not in the breached-password list.
error-E-IDN-008 = The current password is not correct.
error-E-IDN-009 = User { $login } is already active.
error-E-IDN-010 = User { $login } is already inactive.
error-E-WEB-005 = { $entity } does not exist.
error-E-WEB-006 = The system is in read-only mode because disk space is low. Contact the administrator.
error-E-WEB-009 = The request took too long and was stopped. Try again.
error-E-WEB-010 = An internal error occurred. Contact the administrator.
error-E-WEB-011 = The operation { $stage } did not complete. Try again.
error-E-WEB-001 = Another user changed version { $number } of recipe { $reference }. Reload the page and apply your change again.
error-E-DB-001 = Another operation holds a lock on the same data. Try again.
error-E-DB-002 = The operation took too long and was stopped. Try again.
error-E-DB-003 = The change violates a database rule.
error-E-DB-004 = No database connection is free. Try again.
error-E-DB-005 = The database does not answer. Try again.
error-E-DB-006 = The change conflicts with an existing entry.
error-E-DB-007 = The change refers to an entry that does not exist.
error-E-DB-008 = An unexpected database error occurred. Contact the administrator.
error-E-AUD-001 = The change was not saved. The system could not record the audit entry. Contact the administrator.

# Roles
role-administrator = Administrator
role-process-engineer = Process engineer
role-operator = Operator

# The login page
login-title = Log in
login-label-login = Login
login-label-password = Password
login-submit = Log in

# The own account
me-title = My account
me-label-login = Login
me-label-role = Role
me-link-password = Change password
logout-submit = Log out

# The password page
password-title = Change password
password-label-current = Current password
password-label-new = New password
password-hint = Use 12 to 128 characters. Do not use a password from the breached-password list.
password-must-change = You must change the one-time password before you use other pages.
password-submit = Change password
password-changed = The password is changed.

# The user list
users-title = Users
users-search-label = Login contains
users-search-submit = Search
users-new-link = New user
users-empty = No user found.
users-col-login = Login
users-col-role = Role
users-col-status = Status
users-col-password = Password change
users-must-change-yes = Required
users-prev = Previous page
users-next = Next page
user-status-active = Active
user-status-inactive = Inactive

# The new user page
user-new-title = New user
user-new-label-login = Login
user-new-label-role = Role
user-new-hint = A login has 3 to 64 characters from a to z, 0 to 9, period, underscore, and hyphen.
user-new-submit = Create user
user-created-title = User created
user-created = User { $login } is created.

# The user page
user-title = User
user-label-login = Login
user-label-role = Role
user-label-status = Status
user-label-password = Password change
user-label-created = Created
user-label-deactivated = Deactivated
user-label-failures = Failed logins in the window
user-label-last-failure = Last failed login
user-never = None
user-role-label = New role
user-role-submit = Set role
user-deactivate-submit = Deactivate
user-activate-submit = Activate
user-reset-submit = Reset password
user-unlock-submit = Unlock login
user-back-link = Back to the user

# The result of a command
user-password-label = One-time password
user-password-once = Write the password down now. The system shows it only once. The user must change it at the first login.
user-role-changed = The role of { $login } is now { $role }. All sessions of the user ended.
user-deactivated = User { $login } is deactivated. All sessions of the user ended.
user-activated = User { $login } is active.
user-password-reset = The password of { $login } is reset. All sessions of the user ended.
user-unlocked = The login delay of { $login } is cleared.
