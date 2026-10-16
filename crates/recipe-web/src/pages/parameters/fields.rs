//! The reader of the fields of a parameter form (ADR-0039 (a), (b), (e), (f), design D6 and D11).
//!
//! The module solves one problem: a form sends one field per leaf, one field per limit input, and
//! one field per new member leaf. No serde struct can name these keys, so the reader takes the
//! pairs of the body. It decodes one pair at a time, and it counts the pairs and the segments of
//! each leaf path before it parses the path. So a flood of pairs or a long key stops at the first
//! excess with one line `E-WEB-008`.

use std::collections::{BTreeMap, BTreeSet};

use axum::extract::RawForm;
use axum::extract::rejection::RawFormRejection;
use recipe_domain::plant::limits::{MAX_DEPTH, MAX_LEAVES};
use recipe_domain::plant::{LeafClass, LeafPath, LeafSegment, TypeExpr, TypeTree};
use recipe_domain::recipe::{LeafTexts, LimitInput, LimitList, MemberValue, ParameterKey};

use crate::error::{FieldError, FieldReason, WebError};
use crate::hex::hex_value;
use crate::pages::path_values::flag;

/// The key of the session token. The CSRF layer checked it, so the reader drops its value.
const TOKEN_KEY: &str = "csrf_token";
/// The plain field of every form of a draft besides the session token: the row version.
const COMMON_KEYS: [&str; 1] = ["row_version"];
/// The other plain fields of the forms of `AddParameter` and `UpdateParameterDefinition`.
const PARAMETER_KEYS: [&str; 10] = [
    "unit_path",
    "tag",
    "name_en",
    "description_en",
    "type",
    "s7_address",
    "ordinal",
    "value_type",
    "fields",
    "limit_mode",
];
/// The other plain fields of the form of `RebindTypeRevision`.
const REBIND_KEYS: [&str; 2] = ["from_type_id", "to_type_id"];
/// The start of the key of a value field.
const VALUE_PREFIX: &str = "value";
/// The start of the key of a limit field.
const LIMIT_PREFIX: &str = "limits[";
/// The start of the key of a field of a new member leaf.
const MEMBER_PREFIX: &str = "new_member_values[";
/// The largest number of digits of the index of a limit row.
const INDEX_DIGITS: usize = 5;
/// The length of a parameter key in the hyphenated form of a UUID.
const KEY_LENGTH: usize = 36;
/// The largest maximum length of a string leaf: `WSTRING(16382)` (`domain-model.md` section 2.1).
const MAX_LENGTH_LIMIT: u16 = 16_382;
/// The number of pairs per leaf: one value field and four limit fields (ADR-0039 (b)).
const PAIRS_PER_LEAF: usize = 5;
/// The pairs of a form beyond the fields of the leaves (ADR-0039 (b)).
const EXTRA_PAIRS: usize = 64;

/// The fields that one route accepts besides `csrf_token` and `row_version` (`api.md` 2.5, 2.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FieldSet {
    /// `AddParameter` and `UpdateParameterDefinition`: the definition, the values, and the limits.
    Parameter,
    /// `SetParameterValue`: the value fields.
    Value,
    /// `ReplaceLimits`: the limit fields.
    Limits,
    /// `RebindTypeRevision`: the two type identifiers and the fields of the new member leaves.
    Rebind,
}

impl FieldSet {
    /// Return the name of a plain field of the route, or `None` for any other key.
    fn plain(self, key: &str) -> Option<&'static str> {
        let own: &[&'static str] = match self {
            Self::Parameter => &PARAMETER_KEYS,
            Self::Rebind => &REBIND_KEYS,
            Self::Value | Self::Limits => &[],
        };
        COMMON_KEYS
            .iter()
            .chain(own)
            .find(|name| **name == key)
            .copied()
    }

    /// Return true when the route takes the value fields.
    const fn has_values(self) -> bool {
        matches!(self, Self::Parameter | Self::Value)
    }

    /// Return true when the route takes the limit fields.
    const fn has_limits(self) -> bool {
        matches!(self, Self::Parameter | Self::Limits)
    }
}

/// One limit row as the form sent it. The bound texts are not trimmed and not checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SentLimit {
    /// The index `i` of the keys `limits[i].*`. The `id` of each input holds it.
    pub(crate) index: usize,
    /// The leaf path of the row. It can hold `[*]`.
    pub(crate) leaf_path: LeafPath,
    /// The text of the input `min`. A missing key gives an empty text.
    pub(crate) min: String,
    /// The text of the input `max`. A missing key gives an empty text.
    pub(crate) max: String,
    /// The text of the input `max_length`. A missing key gives an empty text.
    pub(crate) max_length: String,
}

/// The fields of a parameter form after the check of the keys (ADR-0039 (b)).
///
/// The texts are as the form sent them. A form that the server renders again shows them.
#[derive(Clone, Debug)]
pub(crate) struct SentFields {
    plain: BTreeMap<&'static str, String>,
    values: Vec<(LeafPath, String)>,
    limits: Vec<SentLimit>,
    members: BTreeMap<(ParameterKey, LeafPath), String>,
    fields: bool,
    limit_mode: Option<LeafPath>,
    value_type: Option<TypeExpr>,
}

/// Return the largest number of pairs of one form: 5 × `budget.leaves_per_parameter` + 64.
pub(crate) fn max_pairs() -> usize {
    usize::try_from(MAX_LEAVES)
        .ok()
        .and_then(|leaves| leaves.checked_mul(PAIRS_PER_LEAF))
        .and_then(|pairs| pairs.checked_add(EXTRA_PAIRS))
        .unwrap_or(usize::MAX)
}

/// One pair of an urlencoded body before the decode: the raw name and the raw value.
pub(crate) type RawPair<'b> = (&'b [u8], &'b [u8]);

/// Return the pairs of an `application/x-www-form-urlencoded` body, one pair at each step.
///
/// The iterator solves one problem: a body of many pairs must not become a list before the reader
/// counts the pairs (ADR-0039 (b)). A step splits one pair and allocates nothing. An empty pair
/// between two `&` is no pair, as in the reader of `axum::Form`.
pub(crate) fn form_pairs(body: &[u8]) -> impl Iterator<Item = RawPair<'_>> {
    body.split(|byte| *byte == b'&')
        .filter(|pair| !pair.is_empty())
        .map(split_pair)
}

/// Split `name=value` at the first `=`. A pair without `=` has an empty value.
fn split_pair(pair: &[u8]) -> RawPair<'_> {
    match pair.iter().position(|byte| *byte == b'=') {
        Some(at) => (
            pair.get(..at).unwrap_or_default(),
            pair.get(at.saturating_add(1)..).unwrap_or_default(),
        ),
        None => (pair, &[]),
    }
}

/// Decode `+` and `%XX` in one raw name or value.
///
/// A `%` that no two hex digits follow stays as it is. Bytes that are not UTF-8 become U+FFFD,
/// as in the reader of `axum::Form`.
fn decode(raw: &[u8]) -> String {
    let mut out = Vec::with_capacity(raw.len());
    let mut bytes = raw.iter().copied();
    while let Some(byte) = bytes.next() {
        match byte {
            b'+' => out.push(b' '),
            b'%' => {
                let mut ahead = bytes.clone();
                let decoded = ahead
                    .next()
                    .and_then(hex_value)
                    .zip(ahead.next().and_then(hex_value))
                    .and_then(|(high, low)| high.checked_mul(16)?.checked_add(low));
                if let Some(decoded) = decoded {
                    out.push(decoded);
                    bytes = ahead;
                } else {
                    out.push(byte);
                }
            }
            other => out.push(other),
        }
    }
    String::from_utf8(out)
        .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned())
}

/// Read the body of a parameter form that `RawForm` buffered (ADR-0039 (b)).
///
/// `RawForm` refuses a content type other than `application/x-www-form-urlencoded`. The reader
/// then decodes the pairs one at a time, so it decodes no pair after the first excess.
///
/// # Errors
///
/// Returns `422 E-WEB-008` with the reason `Form` for a body that `RawForm` refuses, and the errors
/// of [`read_pairs`].
pub(crate) fn read_form(
    form: Result<RawForm, RawFormRejection>,
    set: FieldSet,
) -> Result<SentFields, WebError> {
    let RawForm(body) = form.map_err(|_rejection| bad("form", FieldReason::Form))?;
    read_body(&body, set)
}

/// Read the pairs of the urlencoded `body` of a parameter form with [`read_pairs`].
///
/// # Errors
///
/// Returns the errors of [`read_pairs`].
pub(crate) fn read_body(body: &[u8], set: FieldSet) -> Result<SentFields, WebError> {
    read_pairs(form_pairs(body), set)
}

/// Read the raw pairs of the body of a parameter form.
///
/// The reader counts each pair before it decodes the pair. It stops at the first bad key and
/// answers one line `E-WEB-008`. A pair above [`max_pairs`] gives the reason `Form`. An unknown
/// key and a plain field twice give `Form`. A bad value key or member key gives `ValueKey`, and a
/// bad limit key gives `LimitRow`.
///
/// # Errors
///
/// Returns the `WebError::BadRequest` described above. A flag `fields` other than `1` gives
/// `Flag`. A bad `limit_mode` or `value_type` gives `Reload` with its own field name.
pub(crate) fn read_pairs<'b>(
    pairs: impl IntoIterator<Item = RawPair<'b>>,
    set: FieldSet,
) -> Result<SentFields, WebError> {
    read_each(pairs, |parts, (key, value)| {
        parts.place(set, &decode(key), &decode(value))
    })
}

/// Read pairs that are decoded already, with the rules of [`read_pairs`]. Tests build such pairs.
///
/// # Errors
///
/// Returns the errors of [`read_pairs`].
#[cfg(test)]
pub(crate) fn read<'a>(
    pairs: impl IntoIterator<Item = &'a (String, String)>,
    set: FieldSet,
) -> Result<SentFields, WebError> {
    read_each(pairs, |parts, (key, value)| parts.place(set, key, value))
}

/// Count each pair, stop at the first pair above [`max_pairs`], and put the others in place.
fn read_each<P>(
    pairs: impl IntoIterator<Item = P>,
    place: impl Fn(&mut Parts, P) -> Result<(), WebError>,
) -> Result<SentFields, WebError> {
    let limit = max_pairs();
    let mut parts = Parts::default();
    for (count, pair) in pairs.into_iter().enumerate() {
        if count >= limit {
            return Err(bad("form", FieldReason::Form));
        }
        place(&mut parts, pair)?;
    }
    parts.finish()
}

/// Build the line `E-WEB-008` of `field` with `reason`.
const fn bad(field: &'static str, reason: FieldReason) -> WebError {
    WebError::BadRequest { field, reason }
}

/// Parse a leaf path of a key or a field. It counts the segments before it parses the text.
fn leaf_path(text: &str, reason: FieldReason) -> Result<LeafPath, WebError> {
    let segments = text
        .bytes()
        .filter(|byte| *byte == b'.' || *byte == b'[')
        .count();
    let depth = usize::try_from(MAX_DEPTH).unwrap_or(usize::MAX);
    if segments > depth {
        return Err(bad("form", reason));
    }
    LeafPath::parse(text).map_err(|_reason| bad("form", reason))
}

/// Return true when the path holds the segment `[*]`.
fn has_all(path: &LeafPath) -> bool {
    path.segments().contains(&LeafSegment::All)
}

/// The inputs of one limit row. A key that the body lacks leaves its slot empty.
#[derive(Default)]
struct RowParts {
    leaf_path: Option<String>,
    min: Option<String>,
    max: Option<String>,
    max_length: Option<String>,
}

/// The pairs that the reader has placed so far.
#[derive(Default)]
struct Parts {
    plain: BTreeMap<&'static str, String>,
    values: Vec<(LeafPath, String)>,
    value_paths: BTreeSet<LeafPath>,
    limits: BTreeMap<usize, RowParts>,
    members: BTreeMap<(ParameterKey, LeafPath), String>,
    has_token: bool,
}

impl Parts {
    /// Put one pair into its place, or return the error of its key.
    ///
    /// The value of the session token is not kept, so no `SentFields` holds a secret. A second
    /// token is a form error, as a second plain field is.
    fn place(&mut self, set: FieldSet, key: &str, value: &str) -> Result<(), WebError> {
        if key == TOKEN_KEY {
            if self.has_token {
                return Err(bad("form", FieldReason::Form));
            }
            self.has_token = true;
            return Ok(());
        }
        if let Some(name) = set.plain(key) {
            if self.plain.insert(name, value.to_owned()).is_some() {
                return Err(bad("form", FieldReason::Form));
            }
            return Ok(());
        }
        if set.has_values()
            && let Some(rest) = key.strip_prefix(VALUE_PREFIX)
            && (rest.is_empty() || rest.starts_with(['.', '[']))
        {
            return self.place_value(rest, value);
        }
        if set.has_limits()
            && let Some(rest) = key.strip_prefix(LIMIT_PREFIX)
        {
            return self.place_limit(rest, value);
        }
        if set == FieldSet::Rebind
            && let Some(rest) = key.strip_prefix(MEMBER_PREFIX)
        {
            return self.place_member(rest, value);
        }
        Err(bad("form", FieldReason::Form))
    }

    /// Put the text of one value field into the list. A path with `[*]` or a path twice is bad.
    fn place_value(&mut self, path_text: &str, value: &str) -> Result<(), WebError> {
        let path = leaf_path(path_text, FieldReason::ValueKey)?;
        if has_all(&path) || !self.value_paths.insert(path.clone()) {
            return Err(bad("form", FieldReason::ValueKey));
        }
        self.values.push((path, value.to_owned()));
        Ok(())
    }

    /// Put the text of one limit field `limits[<i>].<field>` into its row.
    fn place_limit(&mut self, rest: &str, value: &str) -> Result<(), WebError> {
        let malformed = || bad("form", FieldReason::LimitRow);
        let (index, field) = rest.split_once("].").ok_or_else(malformed)?;
        let index = row_index(index).ok_or_else(malformed)?;
        let parts = self.limits.entry(index).or_default();
        let slot = match field {
            "leaf_path" => &mut parts.leaf_path,
            "min" => &mut parts.min,
            "max" => &mut parts.max,
            "max_length" => &mut parts.max_length,
            _other => return Err(malformed()),
        };
        if slot.is_some() {
            return Err(malformed());
        }
        *slot = Some(value.to_owned());
        Ok(())
    }

    /// Put the text of one field `new_member_values[<key>]<leaf path>` into the map.
    fn place_member(&mut self, rest: &str, value: &str) -> Result<(), WebError> {
        let malformed = || bad("form", FieldReason::ValueKey);
        let (key, path_text) = rest.split_once(']').ok_or_else(malformed)?;
        if key.len() != KEY_LENGTH {
            return Err(malformed());
        }
        let key = ParameterKey::parse(key).map_err(|_reason| malformed())?;
        let path = leaf_path(path_text, FieldReason::ValueKey)?;
        if has_all(&path) {
            return Err(malformed());
        }
        match self.members.insert((key, path), value.to_owned()) {
            Some(_twice) => Err(malformed()),
            None => Ok(()),
        }
    }

    /// Check the limit rows and the three form flags, and return the fields.
    fn finish(self) -> Result<SentFields, WebError> {
        let limits = limit_rows(self.limits)?;
        let fields = flag("form", self.plain.get("fields").map(String::as_str))?;
        let limit_mode = self
            .plain
            .get("limit_mode")
            .map(|text| level(text))
            .transpose()?;
        let value_type = self
            .plain
            .get("value_type")
            .map(|text| {
                TypeExpr::parse(text).map_err(|_error| bad("value_type", FieldReason::Reload))
            })
            .transpose()?;
        Ok(SentFields {
            plain: self.plain,
            values: self.values,
            limits,
            members: self.members,
            fields,
            limit_mode,
            value_type,
        })
    }
}

/// Read the index of a limit key: one to five digits with no zero in front, except `0`.
fn row_index(text: &str) -> Option<usize> {
    let canonical = (1..=INDEX_DIGITS).contains(&text.len())
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    canonical.then(|| text.parse().ok()).flatten()
}

/// Read the value of the button `limit_mode`: the leaf path of an array level, ending in `[*]`.
fn level(text: &str) -> Result<LeafPath, WebError> {
    let reload = || bad("limit_mode", FieldReason::Reload);
    let path = leaf_path(text, FieldReason::Reload).map_err(|_error| reload())?;
    if path.segments().last() == Some(&LeafSegment::All) {
        Ok(path)
    } else {
        Err(reload())
    }
}

/// Return true when the text is empty after a trim.
fn is_blank(text: Option<&String>) -> bool {
    text.is_none_or(|text| text.trim().is_empty())
}

/// Turn the parts of the limit rows into rows. A blank row without a path is no row.
///
/// A row with a bound and no path, and a row with a malformed path, give `LimitRow`.
fn limit_rows(rows: BTreeMap<usize, RowParts>) -> Result<Vec<SentLimit>, WebError> {
    let mut found = Vec::with_capacity(rows.len());
    for (index, parts) in rows {
        let blank = is_blank(parts.min.as_ref())
            && is_blank(parts.max.as_ref())
            && is_blank(parts.max_length.as_ref());
        let Some(path_text) = parts.leaf_path else {
            if blank {
                continue;
            }
            return Err(bad("form", FieldReason::LimitRow));
        };
        found.push(SentLimit {
            index,
            leaf_path: leaf_path(&path_text, FieldReason::LimitRow)?,
            min: parts.min.unwrap_or_default(),
            max: parts.max.unwrap_or_default(),
            max_length: parts.max_length.unwrap_or_default(),
        });
    }
    Ok(found)
}

/// Return the trimmed text, or `None` when it is empty after the trim.
fn present(text: &str) -> Option<String> {
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// Read a maximum length: one to five digits with a value from 1 to `MAX_LENGTH_LIMIT`.
fn max_length(text: &str) -> Option<u16> {
    let is_digits =
        (1..=INDEX_DIGITS).contains(&text.len()) && text.bytes().all(|byte| byte.is_ascii_digit());
    let length: u16 = is_digits.then(|| text.parse().ok()).flatten()?;
    (1..=MAX_LENGTH_LIMIT).contains(&length).then_some(length)
}

/// Return the text of a leaf as the command takes it (ADR-0039 (e)).
///
/// A `String` or `Char` leaf keeps its text. Every other class trims it. A path that the tree does
/// not hold keeps its text, because the command refuses it with `E-RCP-005`.
fn leaf_text(tree: &TypeTree, path: &LeafPath, text: &str) -> String {
    match path.resolve(tree).map(|target| target.leaf_class()) {
        Some(LeafClass::String | LeafClass::Char) | None => text.to_owned(),
        Some(_trimmed) => text.trim().to_owned(),
    }
}

impl SentFields {
    /// Return the text of the plain field `name`, or `None` when the form did not send it.
    pub(crate) fn text(&self, name: &str) -> Option<&str> {
        self.plain.get(name).map(String::as_str)
    }

    /// Return the value fields in the order of the form, with their texts as sent.
    pub(crate) fn values(&self) -> &[(LeafPath, String)] {
        &self.values
    }

    /// Return the limit rows in the order of their index, with their texts as sent.
    pub(crate) fn limits(&self) -> &[SentLimit] {
        &self.limits
    }

    /// Return true when the user pressed the button "Show value fields" (ADR-0039 (h)).
    pub(crate) const fn wants_fields(&self) -> bool {
        self.fields
    }

    /// Return the array level of the pressed mode button, or `None` (ADR-0039 (k)).
    pub(crate) const fn limit_mode(&self) -> Option<&LeafPath> {
        self.limit_mode.as_ref()
    }

    /// Return the type of the shown grid from the hidden field `value_type` (ADR-0039 (i)).
    pub(crate) const fn value_type(&self) -> Option<&TypeExpr> {
        self.value_type.as_ref()
    }

    /// Return the value fields as the texts of the leaves of `tree` (ADR-0039 (c), (e)).
    ///
    /// # Errors
    ///
    /// Returns `422 E-WEB-008` with the reason `ValueKey` when the pairs are not leaf texts. The
    /// reader refused such pairs already, so this error does not occur after [`read`].
    pub(crate) fn leaf_texts(&self, tree: &TypeTree) -> Result<LeafTexts, WebError> {
        let pairs = self
            .values
            .iter()
            .map(|(path, text)| (path.clone(), leaf_text(tree, path, text)))
            .collect();
        LeafTexts::parse(pairs).map_err(|_error| bad("form", FieldReason::ValueKey))
    }

    /// Return the limit rows as the list of the command. A blank row is no limit.
    ///
    /// The bound texts are trimmed. Each bad maximum length gives one line under its input.
    ///
    /// # Errors
    ///
    /// Returns one line for each bad maximum length. Without such a line, a list that fails the
    /// list check of the domain gives one line `LimitRow` without an input.
    pub(crate) fn limit_list(&self) -> Result<LimitList, Vec<FieldError>> {
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        for row in &self.limits {
            let length = match present(&row.max_length) {
                None => None,
                Some(text) => match max_length(&text) {
                    Some(length) => Some(length),
                    None => {
                        lines.push(length_line(row));
                        continue;
                    }
                },
            };
            let input = LimitInput {
                leaf_path: row.leaf_path.clone(),
                min: present(&row.min),
                max: present(&row.max),
                max_length: length,
            };
            if input.min.is_some() || input.max.is_some() || input.max_length.is_some() {
                rows.push(input);
            }
        }
        if !lines.is_empty() {
            return Err(lines);
        }
        LimitList::parse(rows)
            .map_err(|_reason| vec![FieldError::bad_request("form", FieldReason::LimitRow)])
    }

    /// Return the fields of the new member leaves as the texts of the rebind (ADR-0039 (f)).
    ///
    /// `tree_of` returns the tree of the new revision of a parameter. It sets the class, so the
    /// text of a `String` or `Char` leaf stays as sent. A leaf without a tree keeps its text.
    pub(crate) fn member_values<'t>(
        &self,
        tree_of: &dyn Fn(&ParameterKey) -> Option<&'t TypeTree>,
    ) -> BTreeMap<(ParameterKey, LeafPath), MemberValue> {
        self.members
            .iter()
            .map(|((key, path), text)| {
                let text = match tree_of(key) {
                    Some(tree) => leaf_text(tree, path, text),
                    None => text.clone(),
                };
                ((*key, path.clone()), MemberValue::Text(text))
            })
            .collect()
    }
}

/// Build the line of a bad maximum length under the input `limits-<i>-max_length`.
fn length_line(row: &SentLimit) -> FieldError {
    FieldError::bad_input(
        "field-limit-max-length",
        vec![("leaf", row.leaf_path.to_string())],
        FieldReason::MaxLength,
        format!("limits-{}-max_length", row.index),
    )
}
