//! `FieldReason`, why a request field is not valid (`E-WEB-008`, design D13).

use std::fmt;

/// The reason why a request field is not valid.
///
/// The type solves one problem: the reason must reach the page in the UI language, so the
/// error holds a key and not a text. Each variant has one message `reason-<name>` in both
/// bundles. `Display` gives the English text for the log line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldReason {
    /// The form has a missing field, an unknown field, or a wrong content type.
    Form,
    /// The language is not `pl` or `en`.
    Lang,
    /// A login has a wrong length.
    LoginLength,
    /// A login holds a control character.
    LoginControl,
    /// A new login does not match the login grammar.
    LoginGrammar,
    /// A password has a wrong length.
    PasswordLength,
    /// The role is not in the list.
    Role,
    /// The query string has an unknown key or a value that is not text.
    Query,
    /// The search text is not valid.
    Q,
    /// The page is not a whole number from 1.
    Page,
    /// An `X-Forwarded-For` entry is not an IP address.
    Forwarded,
    /// A machine code does not match the grammar of `MachineCode`.
    MachineCode,
    /// A machine name or a unit name is not valid text.
    Name,
    /// The PLC platform is not one of the five platforms.
    Platform,
    /// A type name does not match the grammar of `TypeName`, or it is the name `new`.
    TypeName,
    /// A member name does not match the grammar of `MemberName`.
    MemberName,
    /// A member row has a name and no type, or a type and no name.
    MemberRows,
    /// The form holds more member rows than `type.max_members`.
    MemberCount,
    /// A flag field holds a value other than `1`.
    Flag,
    /// A search text is not empty and is not valid text.
    SearchText,
}

impl FieldReason {
    /// Every reason, in the order of the table in design D13.
    pub const ALL: [Self; 20] = [
        Self::Form,
        Self::Lang,
        Self::LoginLength,
        Self::LoginControl,
        Self::LoginGrammar,
        Self::PasswordLength,
        Self::Role,
        Self::Query,
        Self::Q,
        Self::Page,
        Self::Forwarded,
        Self::MachineCode,
        Self::Name,
        Self::Platform,
        Self::TypeName,
        Self::MemberName,
        Self::MemberRows,
        Self::MemberCount,
        Self::Flag,
        Self::SearchText,
    ];

    /// Return the key of the message of the reason in the bundles.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Form => "reason-form",
            Self::Lang => "reason-lang",
            Self::LoginLength => "reason-login-length",
            Self::LoginControl => "reason-login-control",
            Self::LoginGrammar => "reason-login-grammar",
            Self::PasswordLength => "reason-password-length",
            Self::Role => "reason-role",
            Self::Query => "reason-query",
            Self::Q => "reason-q",
            Self::Page => "reason-page",
            Self::Forwarded => "reason-forwarded",
            Self::MachineCode => "reason-machine-code",
            Self::Name => "reason-name",
            Self::Platform => "reason-platform",
            Self::TypeName => "reason-type-name",
            Self::MemberName => "reason-member-name",
            Self::MemberRows => "reason-member-rows",
            Self::MemberCount => "reason-member-count",
            Self::Flag => "reason-flag",
            Self::SearchText => "reason-search-text",
        }
    }
}

impl fmt::Display for FieldReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Form => "the form has a missing field, an unknown field, or a wrong content type",
            Self::Lang => "the language must be pl or en",
            Self::LoginLength => "a login has 1 to 64 characters",
            Self::LoginControl => "a login holds no control character",
            Self::LoginGrammar => {
                "a login is 3 to 64 characters from a to z, 0 to 9, period, underscore, and hyphen"
            }
            Self::PasswordLength => "a password has 1 to 128 characters",
            Self::Role => "a role is administrator, process_engineer, or operator",
            Self::Query => "the query string has an unknown key or a key that is not text",
            Self::Q => {
                "the search text has at most 64 characters from a to z, 0 to 9, period, underscore, and hyphen"
            }
            Self::Page => "the page is a whole number from 1",
            Self::Forwarded => "every entry must be an IP address",
            Self::MachineCode => "a machine code is M and a number from 1 to 9999999",
            Self::Name => "a name has 1 to 200 characters and no control character",
            Self::Platform => "choose a PLC platform from the list",
            Self::TypeName => {
                "a type name has 1 to 64 letters, digits, and underscores and starts with a letter or an underscore. It is not new or a primitive name"
            }
            Self::MemberName => {
                "a member name has 1 to 64 letters, digits, and underscores and starts with a letter or an underscore"
            }
            Self::MemberRows => {
                "each member row needs a name and a type. Clear both fields of a row that you do not want"
            }
            Self::MemberCount => "the form holds at most 1024 member rows",
            Self::Flag => "the value must be 1. Reload the page and try again",
            Self::SearchText => {
                "the search text has at most 200 characters and no control character"
            }
        })
    }
}
