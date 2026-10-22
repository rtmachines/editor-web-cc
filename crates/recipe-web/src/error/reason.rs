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
    /// A reference number does not match the grammar of `ReferenceNumber`.
    ReferenceNumber,
    /// A change note is empty, too long, or holds a control character.
    ChangeNote,
    /// A description is empty, too long, or holds a control character.
    Description,
    /// A translated description is too long or holds a control character.
    TranslationDescription,
    /// A parameter tag does not match the grammar of `ParameterTag`.
    Tag,
    /// The order in the unit is not a whole number from 0 to 100000.
    Ordinal,
    /// A PLC address does not match the grammar of `S7Address`.
    S7Address,
    /// The base version is not a version of the list.
    BasedOn,
    /// The state is not a state of the list.
    Status,
    /// A value field names a path twice, names `[*]`, or names no path of the type.
    ValueKey,
    /// A limit row has no path, or it has no minimum, maximum, or maximum length.
    LimitRow,
    /// A maximum length is not a whole number inside the length of the type.
    MaxLength,
    /// The language is not a language of the list.
    LangList,
    /// A hidden field of the form is missing or not valid, so the page must load again.
    Reload,
    /// A filter date does not have the form `YYYY-MM-DD`, or the day does not exist.
    Date,
    /// The filter date `to` is before the filter date `from`.
    DateOrder,
    /// A filter value is not a value of its fixed list.
    Choice,
    /// A filter identifier is not a UUID in the hyphenated form.
    Uuid,
    /// The filter `entity_id` has a value, but the filter `entity_type` has none.
    EntityWithoutType,
    /// The filter `client_ip` is not an IPv4 address or an IPv6 address.
    Address,
    /// The cursor `before` of a list page is not a positive whole number.
    Cursor,
}

impl FieldReason {
    /// Every reason, in the order of the table in design D13. The reasons of the filter inputs
    /// of design D6 come last.
    pub const ALL: [Self; 41] = [
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
        Self::ReferenceNumber,
        Self::ChangeNote,
        Self::Description,
        Self::TranslationDescription,
        Self::Tag,
        Self::Ordinal,
        Self::S7Address,
        Self::BasedOn,
        Self::Status,
        Self::ValueKey,
        Self::LimitRow,
        Self::MaxLength,
        Self::LangList,
        Self::Reload,
        Self::Date,
        Self::DateOrder,
        Self::Choice,
        Self::Uuid,
        Self::EntityWithoutType,
        Self::Address,
        Self::Cursor,
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
            Self::ReferenceNumber => "reason-reference-number",
            Self::ChangeNote => "reason-change-note",
            Self::Description => "reason-description",
            Self::TranslationDescription => "reason-translation-description",
            Self::Tag => "reason-tag",
            Self::Ordinal => "reason-ordinal",
            Self::S7Address => "reason-s7-address",
            Self::BasedOn => "reason-based-on",
            Self::Status => "reason-status",
            Self::ValueKey => "reason-value-key",
            Self::LimitRow => "reason-limit-row",
            Self::MaxLength => "reason-max-length",
            Self::LangList => "reason-lang-list",
            Self::Reload => "reason-reload",
            Self::Date => "reason-date",
            Self::DateOrder => "reason-date-order",
            Self::Choice => "reason-choice",
            Self::Uuid => "reason-uuid",
            Self::EntityWithoutType => "reason-entity-without-type",
            Self::Address => "reason-address",
            Self::Cursor => "reason-cursor",
        }
    }
}

/// Return the English text of a reason of the recipe forms (design D13).
///
/// A reason of the older forms gives an empty text, because `Display` writes those texts itself.
const fn recipe_text(reason: FieldReason) -> &'static str {
    match reason {
        FieldReason::ReferenceNumber => {
            "a reference number has 1 to 50 characters from A to Z, 0 to 9, underscore, and hyphen. It starts with a letter or a digit"
        }
        FieldReason::ChangeNote => {
            "a change note has 1 to 4000 characters and no control character except a line end"
        }
        FieldReason::Description => {
            "a description has 1 to 4000 characters and no control character except a line end"
        }
        FieldReason::TranslationDescription => {
            "a description has 0 to 4000 characters and no control character except a line end"
        }
        FieldReason::Tag => {
            "a tag has 1 to 64 characters from A to Z, 0 to 9, and underscore. It starts with a letter"
        }
        FieldReason::Ordinal => "the order is a whole number from 0 to 100000",
        FieldReason::S7Address => {
            "a PLC address has the form DB1.DBX0.0, DB1.DBB0, DB1.DBW0, or DB1.DBD0"
        }
        FieldReason::BasedOn => "choose a version from the list",
        FieldReason::Status => "choose a state from the list",
        FieldReason::ValueKey => "each value field names one path of the type once, with no [*]",
        FieldReason::LimitRow => {
            "each limit row has a path and a minimum, a maximum, or a maximum length"
        }
        FieldReason::MaxLength => {
            "the maximum length is a whole number from 1 to the length of the type, at most 16382"
        }
        FieldReason::LangList => "choose a language from the list",
        FieldReason::Reload => "reload the page and try again",
        _older => "",
    }
}

/// Return the English text of a reason of the filter inputs (design D6 and D8).
///
/// Any other reason gives the text of `recipe_text`.
const fn filter_text(reason: FieldReason) -> &'static str {
    match reason {
        FieldReason::Date => "a date has the form YYYY-MM-DD, for example 2026-10-17",
        FieldReason::DateOrder => "the end date is the same as the start date or later",
        FieldReason::Choice => "choose a value from the list",
        FieldReason::Uuid => {
            "an identifier has 36 characters in the form 8-4-4-4-12 of the digits 0 to 9 and the letters a to f"
        }
        FieldReason::EntityWithoutType => "choose an object type for the object",
        FieldReason::Address => "an address is an IPv4 or an IPv6 address, for example 192.0.2.10",
        FieldReason::Cursor => "the link of the page is not valid. Open the first page of the list",
        other => recipe_text(other),
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
            other => filter_text(*other),
        })
    }
}
