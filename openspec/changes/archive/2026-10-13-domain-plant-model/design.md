# Design: domain-plant-model

## Context

See `proposal.md`, section Why. `crates/recipe-domain` holds only `lib.rs` and an empty `[dependencies]` table. The workspace lints of `reliability.md` section 2.1 apply to the crate.

The rules come from these sources. This document does not repeat them.

| Topic | Source |
|---|---|
| Entities, value objects, grammars, base units | `domain-model.md` section 2.1 |
| Invariants I8, I9, I10, I21, I22 | `domain-model.md` section 3 |
| Plant commands and their errors | `domain-model.md` section 5.1, `data-model.md` section 6 |
| Error codes and message templates | `reliability.md` sections 1.1, 1.2 |
| Value object rows, size budgets | `reliability.md` sections 4.3, 4.4 |
| Numeric limits | `reliability.md` section 7.11 |
| Primitive matrix, S7 size, class | `data-model.md` section 7 |
| Shape checks of a type row | `data-model.md` sections 3.3, 3.4 |
| Type identity, S7 layout | ADR-0006, ADR-0017, ADR-0018 |
| Archive rules | ADR-0021 |
| Codes `E-PLT-021`, `E-RCP-054` | ADR-0024 |
| Parse reasons of value objects | ADR-0025 |
| Ranges of `DATE` and `DATE_AND_TIME` | ADR-0026 |

## Goals / Non-Goals

**Goals:**

- A value of a public type is valid by construction. No public function builds an invalid value object, an invalid unit tree, or an invalid type row.
- Every public function returns `Result` or `Option`. No path can panic.

**Non-Goals:**

- No knowledge of the type catalog of a machine beyond the rows that a caller passes in.
- No class-2 lock keys of `domain-model.md` section 5.1. P2.3 adds them.
- No configuration of the limits of `reliability.md` section 7.11. The values are constants.

## Decisions

### D1. Module layout

`recipe-domain` gets these modules: `error_codes`, `text`, `timestamp`, `plant`, and `recipe`. The module `recipe` holds only `ReferenceNumber` in this change. The module `plant` has one file for each value object and one file for `PlantError`.

### D2. Hand-written parsers

Each `parse` is a hand-written scanner over `char_indices` and `str::get`. Each failure gives one typed reason (ADR-0025). `domain-model.md` section 2.1 forbids an external crate for `IecLiteral`. The other parsers use the same approach, so the crate needs no `regex` dependency.

The parser of `TypeExpr` reads the `ARRAY[...] OF ` prefixes in a loop, not by recursion. A long input therefore cannot overflow the stack. Each dimension adds one JSON level, so more than `json.max_depth` dimensions in all give `E-PLT-018` at once.

A number in a grammar can be larger than its Rust type, for example `STRING(99999)`. The parser converts it with `try_from` and returns `E-PLT-013`. A revision above the INTEGER range cannot exist, so it gets the reason for an unknown struct revision.

### D3. Error of a value object (ADR-0025)

A value object with its own code returns `PlantError`. `UnitPath` returns `E-PLT-017` or `E-PLT-002`. `TypeExpr` returns `E-PLT-013` with a `DerivedTypeReason`.

Every other value object returns a reason enum from `parse`. Examples are `MachineCode`, `TypeName`, `MemberName`, `S7Address`, `LeafPath`, and `IecLiteral`. A reason has no `code()` and holds no secret. The caller wraps it into a coded error.

- `recipe-web` wraps it into `E-WEB-008` for a request field and `E-WEB-005` for a path value (`api.md` section 1).
- P2.2 wraps an `IecLiteral` reason into `E-RCP-005` for a value and `E-RCP-006` for a limit.

### D4. Error code registry

`ErrorCode` is a `Copy` newtype over `&'static str`. Only `error_codes.rs` creates a value. The constant table `REGISTRY` holds one row for each code.

A test reads `reliability.md` section 1.2 with `include_str!`. It compares the code, the owner variant, and the message template of each `E-PLT` domain row with the Rust variant. P2.1 adds only the 20 `E-PLT` domain rows. Each later change adds the rows of its own variants (`.claude/rules/rust.md`).

### D5. Machine aggregate and unit tree

`Machine` keeps its units in a `BTreeMap<UnitPath, Unit>`. The order of `UnitPath` is numeric per segment, so a parent comes before its children.

- `Machine::create` builds a new machine with the main unit `U1` (`domain-model.md` section 5.1). The caller supplies the identifiers.
- `TryFrom<StoredMachine>` builds a machine from storage. It checks unique paths, a parent for each unit (I10), and a live parent for each live unit. It also checks at least one live unit of depth 1.
- `Machine::unit` returns `Option<&Unit>`. The use case maps `None` of a URL path to `E-WEB-005` (`api.md` section 1).
- `add_unit` joins the parent path and the segment, then calls `UnitPath::parse`. It first checks that the segment is one segment, so the segment `1.2` gives `E-PLT-017`.
- `archive_unit` returns the identifiers of the unit and its live descendants in `id` order. This is the lock order of `data-model.md` section 6.
- The caller passes the units that a live draft or an approved version uses as `UnitUsage`.
- No method changes `plc_platform` (I21). A `compile_fail` doctest shows this.

A failure of `TryFrom<StoredMachine>` means corrupt storage. It returns `E-PLT-004` for a duplicate path, `E-PLT-003` for a missing parent, and `E-PLT-009` for an archived parent. It returns `E-PLT-008` with the path `U1` when no live main unit exists. P2.4 maps every such failure to `StorageError::Other`.

### D6. Type rows

`PlcType` has private fields and one constructor. The constructor checks the shape rules of `data-model.md` sections 3.3 and 3.4 again in the domain. It also checks the string length per platform and 1 to 1024 struct members. Two members with one name give `E-PLT-021` (ADR-0024). A `compile_fail` doctest shows that no field can change (I22).

A record that breaks a shape rule of `data-model.md` section 3.3 gives `E-PLT-013` with the reason `RowShape`. Only a corrupt stored row or a bug gives this reason, so P2.4 maps it to a storage error before HTTP.

`Primitive` is an enum of the 29 names of `data-model.md` section 7. It gives the `LeafClass` of each name. The constructor rejects a row whose class differs from that value. It also rejects a primitive name that the enum does not hold. A primitive that a migration adds therefore also needs a code change. Its value range is code anyway (`domain-model.md` section 2.1).

### D7. Type tree resolution

`TypeTree::resolve` reads the rows of `PlatformTypes`, the resolved value of `domain-model.md` section 1. It uses an explicit stack (`reliability.md` section 2.2). `PlatformTypes` keeps only the rows of one machine and its platform, and the global rows. Two rows with one identifier give `E-PLT-013`.

1. The first pass marks each struct on the current path and stops at a cycle with `E-PLT-005`.
2. The same pass counts the leaves of each row with checked arithmetic. It stops above the leaf budget before it builds a node.
3. The second pass builds the tree.

A referenced identifier that `PlatformTypes` does not hold gives `E-PLT-013` with the reason "not on the platform". `PlatformTypes` holds only the rows of the platform of the machine, so this result also detects I6.

The depth of a tree is the number of JSON levels of its value (`reliability.md` section 4.4). A struct adds one level. An array adds one level for each dimension, because a two-dimensional array is an array of arrays (`data-model.md` section 9.1). The JSON parser of P2.2 counts the levels in the same way.

### D8. S7 address and layout

`S7Address::parse` accepts what `CHECK parameter_s7_address_format` of `data-model.md` section 3.12 accepts, inside the ranges of `domain-model.md` section 2.1. `Display` writes the numbers without leading zeros.

`S7Width` gives the expected width letter of a tree. P2.2 compares it with the width of an address and returns `E-RCP-054` on a mismatch (ADR-0024).

`S7Layout` computes the size and the start class (bit, byte, or even byte) of a tree. It applies the rules of `domain-model.md` section 2.1. The arithmetic uses `checked_*`, and an overflow returns `E-PLT-018`. The limits of D7 make the overflow unreachable.

### D9. IEC literal

`IecLiteral::parse(ty, text)` takes a `TimeType`. A `TimeType` is a time primitive on one platform, so the range check of the base-unit table needs no other argument. The count is an `i128`, which holds the signed and the unsigned 64-bit ranges. ADR-0026 gives the ranges of `DATE` and `DATE_AND_TIME`. A type that does not exist on the platform has no range.

`TryFrom<(TimeType, i128)>` builds a literal from a stored count, so a page can show it. It applies the checks of `parse` after the grammar. `Display` never fails.

The calendar uses the days-from-civil algorithm with checked integer arithmetic.

### D10. Reference number

`PlantError` variants `E-PLT-011`, `E-PLT-012`, and `E-PLT-014` carry `reference`. `domain-model.md` section 1 permits this value object of another context. P2.1 therefore adds `ReferenceNumber` with its grammar of `reliability.md` section 4.3.

### D11. Text of a name

`Text` in this change holds the rule of the row "Text (names)" of `reliability.md` section 4.3. P2.2 adds the description rows as separate types.

### Signatures

```rust
pub struct ErrorCode(&'static str);
impl PlantError { pub fn code(&self) -> ErrorCode; }
impl UnitPath { pub fn parse(text: &str) -> Result<Self, PlantError>; pub fn parent(&self) -> Option<Self>; }
impl TypeExpr { pub fn parse(text: &str) -> Result<Self, PlantError>; }
impl S7Address { pub fn parse(text: &str) -> Result<Self, S7AddressReason>; pub fn width(&self) -> S7Width; }
impl IecLiteral { pub fn parse(ty: TimeType, text: &str) -> Result<Self, IecLiteralReason>; pub fn count(&self) -> i128; }
impl TryFrom<StoredMachine> for Machine { type Error = PlantError; }
impl Machine {
    pub fn create(id: MachineId, code: MachineCode, name: Text, platform: PlcPlatform, main: (UnitId, Text)) -> Self;
    pub fn unit(&self, path: &UnitPath) -> Option<&Unit>;
    pub fn add_unit(&mut self, id: UnitId, parent: Option<&UnitPath>, segment: &str, name: Text)
        -> Result<&Unit, PlantError>;
    pub fn archive_unit(&mut self, path: &UnitPath, now: Timestamp, usage: &UnitUsage)
        -> Result<Vec<UnitId>, PlantError>;
}
impl TypeTree { pub fn resolve(root: PlcTypeId, types: &PlatformTypes) -> Result<Self, PlantError>; }
impl S7Layout { pub fn of(tree: &TypeTree) -> Result<Self, PlantError>; pub fn size_bytes(&self) -> u32; }
```

## Risks / Trade-offs

- [The test of D4 reads a file outside the crate] → The path starts at `CARGO_MANIFEST_DIR`. The crate has `publish = false`.
- [The layout rules can differ from the Siemens compiler] → A golden test holds the sizes that the rules of `domain-model.md` section 2.1 give. ADR-0017 asks for a reference table from the Siemens manual. The repository has no such table yet.
- [The ranges of ADR-0026 come from vendor manuals, not from the draft] → The owner checked them. A wrong range is one row of code and one row of the table.
- [An error variant holds rejected input text without a length cap] → The body limits of `reliability.md` section 7.11 cap the text. Askama escapes it on the page.
- [A Haiku subagent writes the parsers] → Each parser task starts from the grammar table and ends with a property test.

## Open Questions

- Who supplies the Siemens reference table of ADR-0017? The P2.1 golden test does not need it.
