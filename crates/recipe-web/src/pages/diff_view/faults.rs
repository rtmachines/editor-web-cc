//! The field lines of the rule faults in the cells of a diff row (ADR-0118, design D7).

use recipe_domain::recipe::{DefinitionChange, DiffPart, DiffRow, RuleField};

use super::{Cell, LineView, RowContext};
use crate::error::FieldError;

/// The column of the old side in a row that has a tag cell, a name cell, and two sides.
const OLD_COLUMN: usize = 2;
/// The column of the new side in the same row.
const NEW_COLUMN: usize = 3;
/// The column of the value in the row of an added or a removed parameter.
const PARAMETER_VALUE_COLUMN: usize = 4;

/// Return `cell` with the field lines of the rule faults that name its side and its row.
///
/// The problem: a stored limit or value can break the current rule set only (ADR-0118). The diff
/// then keeps the stored text and shows the line of the fault in the cell of the side that holds
/// the part. A row with two sides gets lines in the column of each side. The row of an added or a
/// removed parameter gets lines in its value column, from the one side that holds the parameter.
/// The lines use the mapping of the form errors, so the sentence equals the one of the parameter
/// table.
pub(super) fn fault_cell(
    ctx: &RowContext<'_, '_>,
    (row, part): (&DiffRow, &DiffPart),
    (column, row_id): (usize, &str),
    cell: Cell,
) -> Cell {
    let side = match (part, column) {
        (DiffPart::Parameter { .. }, PARAMETER_VALUE_COLUMN) if row.is_in_new_version() => "new",
        (DiffPart::Parameter { .. }, PARAMETER_VALUE_COLUMN) => "old",
        (DiffPart::Parameter { .. }, _other) => return cell,
        (_two_sides, OLD_COLUMN) => "old",
        (_two_sides, NEW_COLUMN) => "new",
        _other => return cell,
    };
    let Some(faults) = ctx.page.faults_of(row.key()) else {
        return cell;
    };
    let faults = if side == "old" {
        &faults.old
    } else {
        &faults.new
    };
    let lines = faults
        .iter()
        .filter(|fault| names_part(fault.field(), part))
        .map(|fault| ctx.t.line(&FieldError::recipe(fault.error(), None)))
        .collect();
    with_lines(cell, format!("{row_id}-{side}"), lines)
}

/// Return `true` when the field of a rule fault is the part that the row shows.
///
/// A limit row shows the bounds of one stored leaf path. A value row shows one concrete leaf. The
/// row of the S7 address shows the address. The row of an added or a removed parameter shows its
/// scalar value, which is the root leaf. Any other row shows no part that a fault names.
fn names_part(field: &RuleField, part: &DiffPart) -> bool {
    match (part, field) {
        (DiffPart::Limit { path, .. }, RuleField::LimitMin(leaf) | RuleField::LimitMax(leaf))
        | (DiffPart::Value { path, .. }, RuleField::Value(leaf)) => leaf == path,
        (
            DiffPart::Parameter {
                scalar: Some(_), ..
            },
            RuleField::Value(leaf),
        ) => leaf.is_empty(),
        (DiffPart::Definition(DefinitionChange::S7Address { .. }), RuleField::Address) => true,
        _other => false,
    }
}

/// Return `cell` with the field lines `lines` under its text, if there are any (ADR-0118).
///
/// `id` names the block of the lines. A cell without a rule fault stays as it is.
fn with_lines(cell: Cell, id: String, lines: Vec<LineView>) -> Cell {
    if lines.is_empty() {
        return cell;
    }
    Cell {
        lines,
        lines_id: id,
        ..cell
    }
}
