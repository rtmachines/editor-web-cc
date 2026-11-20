//! The `STRING` encoding form of the machine page (ADR-0202, ADR-0041).

use recipe_domain::plant::{MachineCode, StringEncoding};

use crate::error::FormErrors;

/// One option of the select of the encodings. The text is the name of the encoding.
pub(super) struct EncodingOption {
    /// The sent value.
    pub(super) value: &'static str,
    /// The shown text.
    pub(super) text: &'static str,
}

/// The data of the encoding form. The machine page and each answer of the route show it.
///
/// The type solves one problem: the page and the answer of `POST /machines/{code}/string-encoding`
/// render the same block of `machine.html` from one value (ADR-0041 (e)).
pub(super) struct StringEncodingForm {
    /// The address of the route, for example `/machines/M9/string-encoding`.
    pub(super) action: String,
    /// The two encodings in the order of `StringEncoding::ALL`.
    pub(super) options: [EncodingOption; 2],
    /// The option that the select marks: the current encoding or the sent text.
    pub(super) selected: String,
    /// The CSRF token of the session.
    pub(super) csrf_token: String,
    /// True when the page or the answer shows the form. The machine page sets it by permission,
    /// by platform, by version, and by mode.
    pub(super) shown: bool,
    /// True when the answer swaps the form out of band (ADR-0041 (e)).
    pub(super) oob: bool,
    /// The error lines under the select.
    pub(super) errors: FormErrors,
}

impl StringEncodingForm {
    /// Build the form of the machine `code` with the option `selected`. The form is shown and has no line.
    pub(super) fn new(code: MachineCode, selected: String, csrf_token: String) -> Self {
        Self {
            action: format!("/machines/{code}/string-encoding"),
            options: StringEncoding::ALL.map(|encoding| EncodingOption {
                value: encoding.as_str(),
                text: encoding.as_str(),
            }),
            selected,
            csrf_token,
            shown: true,
            oob: false,
            errors: FormErrors::default(),
        }
    }

    /// Return the form with `shown` set by the permission, the platform, the version, and the mode.
    #[must_use]
    pub(super) fn shown(self, shown: bool) -> Self {
        Self { shown, ..self }
    }

    /// Return the form that an answer swaps out of band, with the error lines `errors`.
    pub(super) fn out_of_band(self, errors: FormErrors) -> Self {
        Self {
            oob: true,
            errors,
            ..self
        }
    }
}
