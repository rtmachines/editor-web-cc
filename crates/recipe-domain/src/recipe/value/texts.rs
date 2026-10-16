//! The texts of the leaves of a value (ADR-0039 (c)).
//!
//! A form sends one text per leaf. [`LeafTexts`] holds these texts, [`ParameterValue::from_texts`]
//! builds the value, and [`ParameterValue::texts`] writes the texts of a stored value for a page.

use std::collections::BTreeMap;
use std::fmt;

use serde_json::{Number, Value};

use super::ParameterValue;
use super::check::ordered_faults;
use super::decimal::is_json_number;
use super::expected::Expected;
use super::fault::{ValueFault, ValueFaults};
use super::leaf::Mode;
use super::leaves::{ValueLeaf, value_leaves};
use super::numeric::is_integer;
use super::request_leaf::request_text;
use super::walk::canonicalize;
use crate::plant::{
    IecLiteral, LeafClass, LeafPath, LeafSegment, PlcPlatform, Primitive, TimeType, TypeNode,
    TypeTree,
};
use crate::recipe::limit::ProcessLimit;
use crate::recipe::rebind::build_value;

/// The text of one leaf of a value, as a page shows it (ADR-0039 (c)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafText {
    /// The concrete path of the leaf.
    pub path: LeafPath,
    /// The class of the leaf. It sets the form of the text.
    pub class: LeafClass,
    /// The canonical decimal text, `true` or `false`, the string, or the canonical IEC literal.
    pub text: String,
}

/// Reason why a list of pairs is not a [`LeafTexts`] (ADR-0025). The enum has no code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeafTextsError {
    /// Two pairs name one path.
    DuplicatePath {
        /// The path of both pairs.
        path: LeafPath,
    },
    /// A path holds `[*]`. A text belongs to one leaf.
    AllElements {
        /// The path with the `[*]` segment.
        path: LeafPath,
    },
}

impl fmt::Display for LeafTextsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePath { path } => write!(f, "two value fields name the path {path}"),
            Self::AllElements { path } => {
                write!(f, "the path {path} names every element of an array")
            }
        }
    }
}

/// The texts of the leaves of one value, one text per leaf path, in the order of the form.
///
/// The type solves one problem: the draft gets the fields of a form as checked pairs. No path
/// occurs twice, and no path holds `[*]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafTexts {
    pairs: Vec<(LeafPath, String)>,
    positions: BTreeMap<LeafPath, usize>,
}

impl LeafTexts {
    /// Check the pairs of a form and keep their order.
    ///
    /// # Errors
    ///
    /// Returns [`LeafTextsError::AllElements`] for a path with `[*]`.
    /// Returns [`LeafTextsError::DuplicatePath`] for a path that two pairs name.
    pub fn parse(pairs: Vec<(LeafPath, String)>) -> Result<Self, LeafTextsError> {
        let mut positions = BTreeMap::new();
        for (position, (path, _)) in pairs.iter().enumerate() {
            if path.segments().contains(&LeafSegment::All) {
                return Err(LeafTextsError::AllElements { path: path.clone() });
            }
            if positions.insert(path.clone(), position).is_some() {
                return Err(LeafTextsError::DuplicatePath { path: path.clone() });
            }
        }
        Ok(Self { pairs, positions })
    }

    /// Return the number of pairs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Return `true` when the form sent no pair.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// Iterate over the pairs in the order of the form.
    pub fn iter(&self) -> impl Iterator<Item = (&LeafPath, &str)> {
        self.pairs.iter().map(|(path, text)| (path, text.as_str()))
    }

    /// Return the position and the text of a path, or `None` when the form sent no text for it.
    fn text_at(&self, path: &LeafPath) -> Option<(usize, &str)> {
        let position = *self.positions.get(path)?;
        let (_, text) = self.pairs.get(position)?;
        Some((position, text.as_str()))
    }
}

/// The value of a draft command: JSON text or the texts of a form (ADR-0039).
///
/// The type solves one problem: one draft function accepts both forms, and one set of rules
/// checks both. The web layer sends `Texts`. `Json` keeps the rules of `reliability.md`
/// section 4.4.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueInput {
    /// The JSON text of the whole value.
    Json(String),
    /// One text per leaf, as a form sends it.
    Texts(LeafTexts),
}

/// Write the JSON form of a leaf text by the class of its leaf (`domain-model.md` section 2.1).
///
/// A numeric text follows the JSON number grammar with a dot. A bad text of an integer leaf
/// expects an integer of its type. A bool text is `true` or `false`. Every other text stays a
/// string, and the conversion of the leaf checks it.
pub(super) fn leaf_json(text: &str, primitive: Primitive) -> Result<Value, Expected> {
    match primitive.leaf_class() {
        LeafClass::Numeric => {
            let expected = || {
                if is_integer(primitive) {
                    Expected::Integer {
                        type_name: primitive,
                    }
                } else {
                    Expected::Decimal
                }
            };
            if !is_json_number(text) {
                return Err(expected());
            }
            text.parse::<Number>()
                .map(Value::Number)
                .map_err(|_| expected())
        }
        LeafClass::Bool => match text {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Err(Expected::Bool),
        },
        LeafClass::String | LeafClass::Char | LeafClass::Duration | LeafClass::Instant => {
            Ok(Value::String(text.to_owned()))
        }
    }
}

impl ParameterValue {
    /// Build the value of `tree` from the texts of its leaves (ADR-0039 (c)).
    ///
    /// The function walks the leaves of the tree in preorder and converts each text by the
    /// class of its leaf with the rules of [`ParameterValue::parse`]. It collects every fault.
    /// A missing leaf expects [`Expected::Leaf`]. A path that the tree does not hold expects
    /// [`Expected::KnownLeaf`]. The faults come in the preorder of the leaves, then in the
    /// order of the unknown paths.
    ///
    /// # Errors
    ///
    /// Returns every [`ValueFault`] of the texts in one [`ValueFaults`].
    pub fn from_texts(tree: &TypeTree, texts: &LeafTexts) -> Result<Self, ValueFaults> {
        let built = texts_json(tree, texts);
        let mut faults: Vec<ValueFault> = built.bad_leaves.into_iter().map(|(_, f)| f).collect();
        faults.extend(built.unknown);
        if let Some(faults) = ValueFaults::from_vec(faults) {
            return Err(faults);
        }
        canonicalize(built.json, tree, Mode::Stored).map_err(ValueFaults::from)
    }

    /// Build the value of `tree` from the texts of its leaves and check it against `limits`.
    ///
    /// The function returns every fault in one list (ADR-0041 (b)). It checks each leaf that
    /// converted against its limit, also when another leaf did not convert. The order follows
    /// [`ParameterValue::check_all`], and the fault of a leaf that did not convert takes the
    /// place of that leaf in tree order. The faults of the unknown paths come last.
    ///
    /// # Errors
    ///
    /// Returns every [`ValueFault`] of the texts and of the limits in one [`ValueFaults`].
    pub(in crate::recipe) fn from_texts_checked(
        tree: &TypeTree,
        texts: &LeafTexts,
        limits: &[ProcessLimit],
    ) -> Result<Self, ValueFaults> {
        let built = texts_json(tree, texts);
        if built.bad_leaves.is_empty() && built.unknown.is_empty() {
            let value = canonicalize(built.json, tree, Mode::Stored)?;
            value.check_all(tree, limits)?;
            return Ok(value);
        }
        let shape = built.bad_leaves.into_iter().collect();
        let mut faults = ordered_faults(&built.json, tree, limits, shape)?;
        faults.extend(built.unknown);
        Err(ValueFaults::from_vec(faults).unwrap_or_else(|| ValueFault::WrongTree.into()))
    }

    /// Write the text of each leaf of the value for a page (ADR-0039 (c)).
    ///
    /// The leaves come in preorder. A numeric leaf gives the canonical decimal text (A43). A
    /// time leaf gives the canonical IEC literal. The texts build an equal value again with
    /// [`ParameterValue::from_texts`].
    ///
    /// # Errors
    ///
    /// Returns [`ValueFault::WrongTree`] when `tree` is not the tree of the value.
    pub fn texts(&self, tree: &TypeTree) -> Result<Vec<LeafText>, ValueFault> {
        if tree.root_type() != self.root || tree.platform() != self.platform {
            return Err(ValueFault::WrongTree);
        }
        let leaves = value_leaves(&self.json, tree).ok_or(ValueFault::WrongTree)?;
        leaves
            .into_iter()
            .map(|leaf| leaf_text(leaf, tree.platform()))
            .collect()
    }
}

/// The JSON that the texts of a form build, with the faults of the texts.
struct BuiltJson {
    /// The value. A leaf whose text did not convert holds `null`.
    json: Value,
    /// The fault of each leaf whose text is missing or did not convert, in tree order.
    bad_leaves: Vec<(LeafPath, ValueFault)>,
    /// The fault of each text whose path the tree does not hold, in the order of the form.
    unknown: Vec<ValueFault>,
}

/// Walk the leaves of `tree` in preorder and convert the text of each leaf.
fn texts_json(tree: &TypeTree, texts: &LeafTexts) -> BuiltJson {
    let mut bad_leaves = Vec::new();
    let mut used = vec![false; texts.len()];
    let json = build_value(tree.root(), Vec::new(), &mut |segments, node| {
        let path = LeafPath::from_segments(segments.to_vec());
        match leaf_from_text(texts, &mut used, &path, node, tree.platform()) {
            Ok(value) => value,
            Err(expected) => {
                let fault = ValueFault::Mismatch {
                    leaf: path.clone(),
                    expected,
                };
                bad_leaves.push((path, fault));
                Value::Null
            }
        }
    });
    let unknown = unknown_paths(texts, &used).collect();
    BuiltJson {
        json,
        bad_leaves,
        unknown,
    }
}

/// Convert the text of one leaf, and mark the text as used.
fn leaf_from_text(
    texts: &LeafTexts,
    used: &mut [bool],
    path: &LeafPath,
    node: &TypeNode,
    platform: PlcPlatform,
) -> Result<Value, Expected> {
    let Some((position, text)) = texts.text_at(path) else {
        return Err(Expected::Leaf);
    };
    if let Some(flag) = used.get_mut(position) {
        *flag = true;
    }
    request_text(text, node, platform)
}

/// List a fault for each text whose path the walk did not use, in the order of the form.
fn unknown_paths<'a>(
    texts: &'a LeafTexts,
    used: &'a [bool],
) -> impl Iterator<Item = ValueFault> + 'a {
    texts
        .iter()
        .zip(used)
        .filter(|(_, is_used)| !**is_used)
        .map(|((path, _), _)| ValueFault::Mismatch {
            leaf: path.clone(),
            expected: Expected::KnownLeaf,
        })
}

/// Write the text of one canonical leaf.
fn leaf_text(leaf: ValueLeaf<'_>, platform: PlcPlatform) -> Result<LeafText, ValueFault> {
    let class = leaf.primitive.leaf_class();
    let text = match (class, leaf.value) {
        (LeafClass::Numeric, Value::Number(number)) => Some(number.to_string()),
        (LeafClass::Bool, Value::Bool(flag)) => Some(flag.to_string()),
        (LeafClass::String | LeafClass::Char, Value::String(text)) => Some(text.clone()),
        (LeafClass::Duration | LeafClass::Instant, Value::Number(count)) => {
            literal_text(leaf.primitive, platform, count)
        }
        _ => None,
    };
    let text = text.ok_or(ValueFault::WrongTree)?;
    Ok(LeafText {
        path: leaf.path,
        class,
        text,
    })
}

/// Write the canonical IEC literal of a stored count.
fn literal_text(primitive: Primitive, platform: PlcPlatform, count: &Number) -> Option<String> {
    let time_type = TimeType::of(primitive, platform)?;
    let count: i128 = count.to_string().parse().ok()?;
    IecLiteral::try_from((time_type, count))
        .ok()
        .map(|literal| literal.to_string())
}
