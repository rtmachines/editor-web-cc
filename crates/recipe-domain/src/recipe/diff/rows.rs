//! The rows of a diff, grouped by section (ADR-0050 (c), (d), (e)).

use std::fmt;

use crate::plant::{LeafPath, PlcTypeId, S7Address, UnitId};
use crate::recipe::{LangCode, Ordinal, ParameterKey, ParameterTag, Translation};
use crate::text::{LongText, Text};

/// The section of a diff page (ADR-0050 (d)).
///
/// A page shows the sections in the order of [`DiffKind::ALL`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiffKind {
    /// The changed limit rows, also the rows of added and removed parameters.
    Limits,
    /// The changed values, one row for each leaf.
    Values,
    /// The changed fields of the definitions.
    Definitions,
    /// The changed translations, one row for each language.
    Translations,
    /// The parameters only in the new version.
    Added,
    /// The parameters only in the old version.
    Removed,
}

impl DiffKind {
    /// The sections in the order of a page.
    pub const ALL: [Self; 6] = [
        Self::Limits,
        Self::Values,
        Self::Definitions,
        Self::Translations,
        Self::Added,
        Self::Removed,
    ];

    /// Parse the value of the query key `kind`, such as `limits` (ADR-0050 (e)).
    ///
    /// # Errors
    ///
    /// Returns [`DiffKindReason::Unknown`] for any other text.
    pub fn parse(text: &str) -> Result<Self, DiffKindReason> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_str() == text)
            .ok_or(DiffKindReason::Unknown)
    }

    /// Return the value of the query key `kind` for this section.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Limits => "limits",
            Self::Values => "values",
            Self::Definitions => "definitions",
            Self::Translations => "translations",
            Self::Added => "added",
            Self::Removed => "removed",
        }
    }
}

/// Reason why a text is not a section of a diff page.
///
/// The enum has no code. The web layer gives the code of a bad query value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffKindReason {
    /// The text is not one of the six section names.
    Unknown,
}

impl fmt::Display for DiffKindReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => f.write_str("the text is not a section of a diff"),
        }
    }
}

/// The bounds of one limit row as text (ADR-0050 (c)).
///
/// A time bound is the canonical IEC literal. Any other bound is canonical decimal text (A43).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffBounds {
    /// The lower bound, or `None` when the row has none.
    pub min: Option<String>,
    /// The upper bound, or `None` when the row has none.
    pub max: Option<String>,
    /// The largest length of a string leaf, or `None` when the row has none.
    pub max_length: Option<String>,
}

/// One changed field of a definition, with the old and the new content (ADR-0050 (c)).
///
/// The unit and the type are identifiers. The use case gives the unit path and the type text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DefinitionChange {
    /// The tag changed.
    Tag {
        /// The old tag.
        old: ParameterTag,
        /// The new tag.
        new: ParameterTag,
    },
    /// The parameter moved to another unit.
    Unit {
        /// The old unit.
        old: UnitId,
        /// The new unit.
        new: UnitId,
    },
    /// The type changed. The use case writes the full type text of each identifier.
    Type {
        /// The old type.
        old: PlcTypeId,
        /// The new type.
        new: PlcTypeId,
    },
    /// The S7 address changed.
    S7Address {
        /// The old address, or `None` when the parameter had none.
        old: Option<S7Address>,
        /// The new address, or `None` when the parameter has none.
        new: Option<S7Address>,
    },
    /// The position changed.
    Ordinal {
        /// The old position.
        old: Ordinal,
        /// The new position.
        new: Ordinal,
    },
    /// The English name changed.
    NameEn {
        /// The old name.
        old: Text,
        /// The new name.
        new: Text,
    },
    /// The English description changed.
    DescriptionEn {
        /// The old description.
        old: LongText,
        /// The new description.
        new: LongText,
    },
}

/// The content of one row of a diff (ADR-0050 (c), (d), (e)).
///
/// `None` on one side means that this side has no such row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffPart {
    /// A limit row by its stored leaf path. A `[*]` row is one row.
    Limit {
        /// The stored leaf path of the row.
        path: LeafPath,
        /// The bounds of the old version.
        old: Option<DiffBounds>,
        /// The bounds of the new version.
        new: Option<DiffBounds>,
    },
    /// The value of one leaf, with the effective limit of the new version for that leaf.
    Value {
        /// The concrete path of the leaf.
        path: LeafPath,
        /// The text of the old version.
        old: Option<String>,
        /// The text of the new version.
        new: Option<String>,
        /// The limit of the new version that covers the leaf, or `None` when no limit covers it.
        limit: Option<DiffBounds>,
    },
    /// One changed field of the definition.
    Definition(DefinitionChange),
    /// The translation of one language.
    Translation {
        /// The language of the translation.
        lang: LangCode,
        /// The translation of the old version.
        old: Option<Translation>,
        /// The translation of the new version.
        new: Option<Translation>,
    },
    /// An added or a removed parameter (ADR-0050 (d)).
    Parameter {
        /// The English name.
        name_en: Text,
        /// The unit of the parameter.
        unit: UnitId,
        /// The type. The use case writes its full type text.
        plc_type: PlcTypeId,
        /// The text of a scalar value, or `None` for a structured value.
        scalar: Option<String>,
        /// The number of leaves of the value.
        leaves: usize,
    },
    /// The rows of this parameter in this section after the first `diff.leaf_rows` rows.
    Omitted {
        /// The number of rows that the section does not show.
        count: usize,
    },
}

/// One row of a diff section with the parameter that it belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffRow {
    key: ParameterKey,
    tag: ParameterTag,
    part: DiffPart,
    /// `true` when the new version holds the parameter. Only a removed parameter has `false`.
    in_new_version: bool,
}

impl DiffRow {
    /// Return `true` when the new version holds the parameter of the row.
    ///
    /// Each row of a kept parameter returns `true`, also a row that only the old side holds. Only
    /// the rows of a removed parameter return `false` (ADR-0050 (d)).
    #[must_use]
    pub const fn is_in_new_version(&self) -> bool {
        self.in_new_version
    }

    /// Return the key of the parameter.
    #[must_use]
    pub const fn key(&self) -> ParameterKey {
        self.key
    }

    /// Return the tag of the parameter. A removed parameter has the tag of the old version.
    #[must_use]
    pub const fn tag(&self) -> &ParameterTag {
        &self.tag
    }

    /// Return the content of the row.
    #[must_use]
    pub const fn part(&self) -> &DiffPart {
        &self.part
    }
}

/// The rows of one parameter, one list for each section.
#[derive(Debug, Default)]
pub(super) struct PairParts {
    pub(super) limits: Vec<DiffPart>,
    pub(super) values: Vec<DiffPart>,
    pub(super) definitions: Vec<DiffPart>,
    pub(super) translations: Vec<DiffPart>,
    pub(super) added: Vec<DiffPart>,
    pub(super) removed: Vec<DiffPart>,
}

impl PairParts {
    /// Return `true` when the parameter has no row in any section.
    fn is_empty(&self) -> bool {
        [
            &self.limits,
            &self.values,
            &self.definitions,
            &self.translations,
            &self.added,
            &self.removed,
        ]
        .iter()
        .all(|rows| rows.is_empty())
    }

    /// Return `true` when the new version holds the parameter.
    ///
    /// Only `pair_parts` fills `removed`, for a parameter that only the old version holds
    /// (`diff.rs`). An empty list therefore means that the new version holds the parameter.
    fn is_in_new_version(&self) -> bool {
        self.removed.is_empty()
    }
}

/// The rows of the diff of one page, grouped by section (ADR-0050 (d)).
///
/// Inside a section, the rows come in the order of the pairs, then in the order of each part.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiffRows {
    limits: Vec<DiffRow>,
    values: Vec<DiffRow>,
    definitions: Vec<DiffRow>,
    translations: Vec<DiffRow>,
    added: Vec<DiffRow>,
    removed: Vec<DiffRow>,
    unchanged: usize,
}

impl DiffRows {
    /// Return the rows of one section.
    #[must_use]
    pub fn section(&self, kind: DiffKind) -> &[DiffRow] {
        match kind {
            DiffKind::Limits => &self.limits,
            DiffKind::Values => &self.values,
            DiffKind::Definitions => &self.definitions,
            DiffKind::Translations => &self.translations,
            DiffKind::Added => &self.added,
            DiffKind::Removed => &self.removed,
        }
    }

    /// Return the number of pairs on both sides without a difference.
    #[must_use]
    pub const fn unchanged(&self) -> usize {
        self.unchanged
    }

    /// Return `true` when no section holds a row.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        DiffKind::ALL
            .iter()
            .all(|kind| self.section(*kind).is_empty())
    }

    /// Build the rows from the parts of each pair. A pair without a part counts as unchanged.
    pub(super) fn from_parts<I>(parts: I, leaf_rows: usize) -> Self
    where
        I: IntoIterator<Item = (ParameterKey, ParameterTag, PairParts)>,
    {
        let mut rows = Self::default();
        let mut unchanged_flags = Vec::new();
        for (key, tag, pair) in parts {
            unchanged_flags.push(pair.is_empty());
            let in_new_version = pair.is_in_new_version();
            let push = |section: &mut Vec<DiffRow>, parts: Vec<DiffPart>| {
                section.extend(cut(parts, leaf_rows).into_iter().map(|part| DiffRow {
                    key,
                    tag: tag.clone(),
                    part,
                    in_new_version,
                }));
            };
            push(&mut rows.limits, pair.limits);
            push(&mut rows.values, pair.values);
            push(&mut rows.definitions, pair.definitions);
            push(&mut rows.translations, pair.translations);
            push(&mut rows.added, pair.added);
            push(&mut rows.removed, pair.removed);
        }
        rows.unchanged = unchanged_flags.into_iter().filter(|flag| *flag).count();
        rows
    }
}

/// Keep the first `leaf_rows` parts and add the count of the others (ADR-0050 (e)).
fn cut(mut parts: Vec<DiffPart>, leaf_rows: usize) -> Vec<DiffPart> {
    if parts.len() <= leaf_rows {
        return parts;
    }
    let count = parts.split_off(leaf_rows).len();
    parts.push(DiffPart::Omitted { count });
    parts
}
