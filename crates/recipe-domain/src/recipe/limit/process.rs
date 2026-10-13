//! The process limit of one leaf (`domain-model.md` section 2.2, design D9).

use rust_decimal::Decimal;

use crate::plant::{LeafPath, PlcPlatform, PlcTypeId};

/// The bounds that a limit holds, by the class of its leaf.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Bounds {
    /// A numeric, duration, or instant leaf. A time bound is a count of the base unit.
    Range {
        min: Option<Decimal>,
        max: Option<Decimal>,
    },
    /// A string leaf.
    MaxLength(u16),
}

/// A limit on a leaf, or on every element of an array, that passed the checks of its tree.
///
/// The type solves one problem: code cannot hold a limit that its type tree does not accept.
/// Only [`ProcessLimit::parse`] and the stored conversion create it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessLimit {
    root: PlcTypeId,
    platform: PlcPlatform,
    leaf_path: LeafPath,
    bounds: Bounds,
}

impl ProcessLimit {
    /// Build a limit from parts that the caller has checked.
    pub(super) const fn from_checked(
        (root, platform): (PlcTypeId, PlcPlatform),
        leaf_path: LeafPath,
        bounds: Bounds,
    ) -> Self {
        Self {
            root,
            platform,
            leaf_path,
            bounds,
        }
    }

    /// Return the root type of the tree that accepted the limit.
    #[must_use]
    pub const fn root_type(&self) -> PlcTypeId {
        self.root
    }

    /// Return the platform of the tree that accepted the limit.
    #[must_use]
    pub const fn platform(&self) -> PlcPlatform {
        self.platform
    }

    /// Return the path of the limited leaf.
    #[must_use]
    pub const fn leaf_path(&self) -> &LeafPath {
        &self.leaf_path
    }

    /// Return the lower bound. A time bound is a count of the base unit.
    #[must_use]
    pub const fn min(&self) -> Option<Decimal> {
        match self.bounds {
            Bounds::Range { min, .. } => min,
            Bounds::MaxLength(_) => None,
        }
    }

    /// Return the upper bound. A time bound is a count of the base unit.
    #[must_use]
    pub const fn max(&self) -> Option<Decimal> {
        match self.bounds {
            Bounds::Range { max, .. } => max,
            Bounds::MaxLength(_) => None,
        }
    }

    /// Return the largest length of a string leaf.
    #[must_use]
    pub const fn max_length(&self) -> Option<u16> {
        match self.bounds {
            Bounds::Range { .. } => None,
            Bounds::MaxLength(length) => Some(length),
        }
    }

    /// Return the limit in the form that storage keeps (`data-model.md` section 3.13).
    #[must_use]
    pub fn to_stored(&self) -> StoredLimit {
        StoredLimit {
            leaf_path: self.leaf_path.clone(),
            min: self.min(),
            max: self.max(),
            max_length: self.max_length(),
        }
    }
}

/// The row of a limit as storage returns it.
///
/// The row is not checked. The conversion `TryFrom<(&StoredLimit, &TypeTree)>` for
/// [`ProcessLimit`] checks it. A time bound is a count of the base unit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredLimit {
    /// The path of the limited leaf.
    pub leaf_path: LeafPath,
    /// The lower bound, or `None` when the row has none.
    pub min: Option<Decimal>,
    /// The upper bound, or `None` when the row has none.
    pub max: Option<Decimal>,
    /// The largest length of a string leaf, or `None` when the row has none.
    pub max_length: Option<u16>,
}
