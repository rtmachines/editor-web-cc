//! The form of an error answer and the lines under its inputs (design D9, ADR-0041 (e)).

use std::collections::BTreeMap;
use std::fmt;

use askama::Template;
use recipe_domain::audit::RequestId;

use super::lists::line_view;
use super::{FieldError, LineView, NO_LINES, WebError};
use crate::i18n::{Bundles, Lang};
use crate::layout::SessionFrame;

/// The tag that a rendered form starts with.
const FORM_START: &str = "<form id=\"";
/// The attribute that swaps the form out of band. Its value is a literal (ADR-0041 (e)).
const OOB_ATTRIBUTE: &str = " hx-swap-oob=\"true\"";

/// The HTML of a form that an answer swaps out of band (ADR-0041 (e) and (h)).
///
/// The type solves two problems. The form holds the CSRF token, so `Debug` writes only the
/// length. The answer appends the HTML as it is, so only an askama template builds it, and askama
/// escapes every value. The form starts with `<form id="...` and holds the literal
/// `hx-swap-oob="true"` in its first tag.
///
/// A form of a request without HTMX also holds the session parts of the layout. The error layer
/// then shows the form on a full page in the layout of the session (`api.md` section 5.3).
pub struct RenderedForm {
    /// The HTML of the form.
    html: String,
    /// The session parts of the layout of a full page, or `None` for an HTMX answer. The box
    /// keeps `WebError` small (`clippy::result_large_err`).
    frame: Option<Box<SessionFrame>>,
}

impl RenderedForm {
    /// Render the form template `form`.
    ///
    /// # Errors
    ///
    /// Returns `WebError::Internal` (`E-WEB-010`) when the template cannot render, or when the
    /// first tag is not a form with an `id` first and the attribute `hx-swap-oob="true"`.
    pub(crate) fn from_template<T: Template>(
        form: &T,
        request_id: RequestId,
    ) -> Result<Self, WebError> {
        let html = form
            .render()
            .map_err(|_failed| WebError::Internal { request_id })?;
        if is_out_of_band_form(&html) {
            Ok(Self { html, frame: None })
        } else {
            Err(WebError::Internal { request_id })
        }
    }

    /// Return the form with the session parts of the layout of a full page.
    #[must_use]
    pub(crate) fn with_frame(self, frame: Option<SessionFrame>) -> Self {
        Self {
            frame: frame.map(Box::new),
            ..self
        }
    }

    /// Return the HTML of the form.
    pub(crate) fn as_str(&self) -> &str {
        &self.html
    }

    /// Return the session parts of the layout of a full page, if the form has them.
    pub(crate) fn frame(&self) -> Option<&SessionFrame> {
        self.frame.as_deref()
    }

    /// Return the HTML of the form for a full page: a plain form without an `hx-*` attribute.
    ///
    /// The page of a refused form has only the elements of the layout. A target of the page that
    /// sent the form is missing there, so a boosted form would send nothing. A plain form sends a
    /// normal `POST` also while HTMX runs (ADR-0047 (a)). Only the first tag of a form template
    /// holds `hx-*` attributes, and [`Self::from_template`] checks that this tag starts the HTML.
    pub(crate) fn page_html(&self) -> String {
        match self.html.trim_start().split_once('>') {
            Some((tag, body)) => format!("{}>{body}", plain_tag(tag)),
            None => self.html.clone(),
        }
    }
}

/// Return the tag `tag` without its `hx-*` attributes.
///
/// Askama escapes `"` in each value, so each `"` of the tag opens or closes a value. The text
/// before a value ends with the name of its attribute and `=`.
fn plain_tag(tag: &str) -> String {
    let mut plain = String::with_capacity(tag.len());
    let mut segments = tag.split('"');
    while let Some(outside) = segments.next() {
        let Some(value) = segments.next() else {
            plain.push_str(outside);
            break;
        };
        match outside.rsplit_once(' ') {
            Some((kept, name)) if name.starts_with("hx-") => plain.push_str(kept),
            _other => {
                plain.push_str(outside);
                plain.push('"');
                plain.push_str(value);
                plain.push('"');
            }
        }
    }
    plain
}

impl fmt::Debug for RenderedForm {
    /// Write only the length. The form and the session parts hold the CSRF token of the session.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RenderedForm")
            .field("len", &self.html.len())
            .finish()
    }
}

/// Return true when the first tag of `html` is a form with an `id` and the out-of-band attribute.
///
/// Askama escapes `>` and `"` in each value, so the first `>` ends the first tag, and no value can
/// add the attribute.
fn is_out_of_band_form(html: &str) -> bool {
    let Some(rest) = html.trim_start().strip_prefix(FORM_START) else {
        return false;
    };
    rest.split_once('>')
        .is_some_and(|(tag, _body)| tag.contains(OOB_ATTRIBUTE))
}

/// The error lines of a form, by the `id` of their input (ADR-0041 (e)).
///
/// The type solves one problem: a form template shows each line under its input, so it asks for
/// the lines of one `id`. A line without an input stands only in the summary.
#[derive(Default)]
pub(crate) struct FormErrors(BTreeMap<String, Vec<LineView>>);

impl FormErrors {
    /// Group the lines by their input in `lang`, in the order of the list.
    pub(crate) fn new<'a>(
        lines: impl IntoIterator<Item = &'a FieldError>,
        bundles: &Bundles,
        lang: Lang,
    ) -> Self {
        let mut inputs: BTreeMap<String, Vec<LineView>> = BTreeMap::new();
        for line in lines {
            let view = line_view(line, bundles, lang);
            if !view.target.is_empty() {
                inputs.entry(view.target.clone()).or_default().push(view);
            }
        }
        Self(inputs)
    }

    /// Return the lines of the input `id`. An input without an error has none.
    pub(crate) fn lines(&self, id: &str) -> &[LineView] {
        self.0.get(id).map_or(NO_LINES, Vec::as_slice)
    }
}
