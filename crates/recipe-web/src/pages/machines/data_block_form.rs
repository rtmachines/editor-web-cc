//! The data block form of the machine page (design D8, ADR-0156).

use recipe_domain::plant::MachineCode;

use crate::error::FormErrors;
use crate::pages::texts::Texts;

/// The data of the data block form. The machine page and each answer of the route show it.
///
/// The type solves one problem: the page and the error answer of `POST /machines/{code}/data-block`
/// render the same block of `machine.html` from one value (ADR-0041 (e)).
pub(super) struct DataBlockForm {
    /// The address of the route, for example `/machines/M7/data-block`.
    pub(super) action: String,
    /// The number in the input: the current number or the sent text.
    pub(super) db_number: String,
    /// The CSRF token of the session.
    pub(super) csrf_token: String,
    /// True when the page or the answer shows the form. The machine page sets it by permission
    /// and by platform.
    pub(super) shown: bool,
    /// True when the answer swaps the form out of band (ADR-0041 (e)).
    pub(super) oob: bool,
    /// The highest data block number of the platform of the machine. `None` shows no hint.
    pub(super) max: Option<u16>,
    /// The error lines under the input.
    pub(super) errors: FormErrors,
}

impl DataBlockForm {
    /// Build the form of the machine `code` with the text `db_number`. The form is shown and has no line.
    pub(super) fn new(code: MachineCode, db_number: String, csrf_token: String) -> Self {
        Self {
            action: format!("/machines/{code}/data-block"),
            db_number,
            csrf_token,
            shown: true,
            oob: false,
            max: None,
            errors: FormErrors::default(),
        }
    }

    /// Return the form with the highest number of the platform of the machine.
    #[must_use]
    pub(super) fn with_max(self, max: Option<u16>) -> Self {
        Self { max, ..self }
    }

    /// Return the hint under the label: the range of the platform, or an empty text when the
    /// platform is not known.
    ///
    /// The method solves one problem: the hint of the machine form says "leave it empty" and
    /// names both ranges, and this form needs a number in the range of one platform only.
    pub(super) fn hint(&self, t: &Texts<'_>) -> String {
        self.max
            .map(|max| t.with_arg("machine-data-block-hint", "max", max))
            .unwrap_or_default()
    }

    /// Return the form with `shown` set by the permission of the role and the platform.
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
