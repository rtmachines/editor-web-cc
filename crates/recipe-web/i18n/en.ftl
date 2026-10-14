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

# Layout
app-name = Recipe editor
skip-to-content = Skip to the content
nav-label = Main
nav-recipes = Recipes
nav-users = Users
nav-me = My account
lang-switch-label = Language
lang-switch-pl = Polski
lang-switch-en = English
user-signed-in = Signed in as

# Recipe list
recipes-title = Recipes
recipes-empty = No recipe to show.

# States
status-draft = Draft
status-approved = Approved
status-retired = Retired

# Forms
form-required = required

# Error page
error-page-title = The request failed
error-page-home = Open the recipe list

# Field labels
field-form = Form
field-lang = Language
field-login = Login
field-password = Password
field-current-password = Current password
field-role = Role
field-query = Search fields
field-q = Login contains
field-page = Page
field-X-Forwarded-For = Header X-Forwarded-For

# Field reasons
reason-form = the form has a missing field, an unknown field, or a wrong content type
reason-lang = the language must be pl or en
reason-login-length = a login has 1 to 64 characters
reason-login-control = a login holds no control character
reason-login-grammar = a login is 3 to 64 characters from a to z, 0 to 9, period, underscore, and hyphen
reason-password-length = a password has 1 to 128 characters
reason-role = choose a role from the list
reason-query = the address has an unknown parameter or a parameter that is not text
reason-q = the search text has at most 64 characters from a to z, 0 to 9, period, underscore, and hyphen
reason-page = the page is a whole number from 1
reason-forwarded = every entry must be an IP address

# Missing entities
missing-machine = Machine { $code }
missing-machine-id = Machine { $id }
missing-unit = Unit { $path } of machine { $code }
missing-type = Type { $name } of machine { $code }
missing-recipe = Recipe { $reference }
missing-version = Version { $number } of recipe { $reference }
missing-user = User { $login }
missing-malformed-login = User

# Password reasons and task stages
password-reason-empty = it is empty
password-reason-too-short = it has fewer than 12 characters
password-reason-too-long = it has more than 128 characters
password-reason-breached = it is in the breached-password list
task-stage-password-hash = password hash
task-stage-validation = validation
task-stage-export = export
task-stage-token = token generation

# Operations
operation-create-machine = Create machine
operation-rename-machine = Rename machine
operation-archive-machine = Archive machine
operation-add-unit = Add unit
operation-rename-unit = Rename unit
operation-archive-unit = Archive unit
operation-define-udt = Define UDT
operation-create-recipe = Create recipe
operation-rename-recipe = Rename recipe
operation-delete-recipe = Delete recipe
operation-restore-recipe = Restore recipe
operation-create-version = Create version
operation-discard-draft = Discard draft
operation-approve-version = Approve version
operation-retire-version = Retire version
operation-add-parameter = Add parameter
operation-update-parameter-definition = Change parameter definition
operation-set-parameter-value = Set parameter value
operation-replace-limits = Replace limits
operation-upsert-translation = Save translation
operation-remove-translation = Delete translation
operation-remove-parameter = Delete parameter
operation-rebind-type-revision = Rebind type revision
operation-login = Log in
operation-logout = Log out
operation-create-user = Create user
operation-set-user-role = Set user role
operation-deactivate-user = Deactivate user
operation-activate-user = Activate user
operation-reset-password = Reset password
operation-change-own-password = Change own password
operation-unlock-login = Unlock login
operation-export-version = Export version
operation-list-machines = List machines
operation-get-machine = Show machine
operation-get-unit-tree = Show unit tree
operation-get-unit = Show unit
operation-list-types = List types
operation-get-type-tree = Show type tree
operation-get-type-usage = Show type usage
operation-list-recipes = List recipes
operation-get-recipe = Show recipe
operation-list-versions = List versions
operation-get-version = Show version
operation-diff-versions = Compare versions
operation-get-recipe-history = Show recipe history
operation-list-parameters = List parameters
operation-get-parameter = Show parameter
operation-get-limits = Show limits
operation-get-translations = Show translations
operation-list-users = List users
operation-get-user = Show user
operation-list-audit-entries = List audit entries
operation-get-audit-entry = Show audit entry
operation-list-security-events = List security events
operation-get-own-profile = Show own profile
operation-get-machine-form = Open machine form
operation-get-unit-form = Open unit form
operation-get-udt-form = Open UDT form
operation-get-recipe-form = Open recipe form
operation-get-version-form = Open version form
operation-get-parameter-form = Open parameter form
operation-get-approval-preview = Show approval preview
operation-get-user-form = Open user form
operation-get-login-form = Open login form
operation-get-own-password-form = Open password form
