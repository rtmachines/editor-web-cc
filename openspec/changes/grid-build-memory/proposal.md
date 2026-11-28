# Proposal: grid-build-memory

## Why

The largest valid parameter of A26 gives a `GET` render peak of 106 201 524 B. Its refused round 1 form gives 173 268 053 B (ADR-0223). A later bound on parallel grid answers needs at most 49 653 707 B and 60 959 672 B.

The attribution of 2026-11-27 found the cause. The row model holds 0 B of the three profiled peaks. Each peak falls in `LimitSet::new`, before the walk writes one row:

- The two level sets of `LimitSet::new` hold 78 684 957 B.
- Copies of the leaf paths hold 18 874 368 B of the `GET` peak and 75 497 472 B of the refused peak.

The change satisfies R30 and R31 and uses A26.

## What Changes

- C1: `LimitSet::new` keeps no set of index prefixes and buffers no type levels.
- C2a: the save handler drops the leaf texts and the limit list before it builds the refused form.
- C2b: `GridTexts::stored` drops the stored value and the leaf texts before it writes the limit texts.
- C2c: `LimitTexts` holds the text of its path, not a `LeafPath`.
- C2d: `extended` in `recipe-domain` allocates the exact path length.
- C3a: a leaf row holds numbers, flags, and one path. The templates write each `id`, `name`, and hint key from a `Display` view.
- C2e, if the owner accepts it: the refused save drops the leaf texts before the limit list.
- Each task that lowers a peak lowers its pin. ADR-0226 amends ADR-0222, ADR-0223, and ADR-0225.
- Each grid answer keeps its pinned length and SHA-256.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: the requirement "Peak memory of a grid answer" names the SHA-256 pin of each grid answer.

## Impact

- `crates/recipe-web`: the grid modules and tests, `form_view.rs`, `prepare.rs`, `fields.rs`, `components/grid.html`, and seven pin files.
- `crates/recipe-domain`: `plant/leaf_path.rs`.
- `docs/adr`: ADR-0226, its index row, and the decision map.
- No route, setting, error code, migration, or stored data changes.

## Out of scope

- The budget `form.grid_memory`, the write check with `E-WEB-018`, A54, and a bound on parallel `GET` pages (the next change).
- A text path in `SentFields`, and the two page copies of the error layer.
- A lower leaf cap of A26 (option (c)) after a missed target.
- Changes of A26, ADR-0211, and the report-only round 1 of ADR-0225.
