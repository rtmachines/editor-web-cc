# ADR-0211: Length of a leaf path, the body limit, and the permits of the value routes

Status: Accepted. Amends ADR-0039 (g)
Date: 2026-11-22
Deciders: project owner

## Context

ADR-0039 (a) names each value field `value` followed by the text of the `LeafPath`. ADR-0039 (g) set `http.body_limit.value` to 4 MiB. That calculation did not count the length of the leaf path.

The edit form of a parameter sends the leaf path twice for each leaf with a limit. The name of the value field holds it. The hidden field `limits[<i>].leaf_path` holds it as its value. The code is in `crates/recipe-web/src/pages/parameters/grid/walk.rs`.

No document limits the length of a leaf path. The grammar of `LeafPath` (`domain-model.md` §2.1) permits member names of 64 characters and `json.max_depth` (32) segments.

These facts give a failure. A struct has four nested members with names of 64 characters, and the last member is `ARRAY[0..16383] OF INT`. The JSON value is below 40 KiB, but the value fields alone exceed 4 194 304 bytes. The save gets `413 E-WEB-003` before validation. The value is valid under every budget of A26, and the UI has no other way to save it.

A larger body limit lets each request buffer more memory. The value routes have no limit on the count of such requests at the same time.

## Decision drivers

- A value inside the budgets of A26 and `budget.value_bytes` fits the body limit of its route (ADR-0039 drivers, R30).
- A stored type stays readable. A new limit applies only where a command creates a type row (R13, R38).
- The buffered bodies of the value routes stay inside `container.app_memory` (R30).
- The limit mode of ADR-0039 (k) keeps the leaf path of each limit.
- Each numeric setting comes from `reliability.md` §7.11.

## Considered options

### Option A: A leaf path of at most 128 bytes, a body limit of 16 MiB, and permits

- Pros: the field names of ADR-0039 stay. The largest edit form needs at most 15.2 MiB. The permits limit the memory of the buffered bodies.
- Cons: a new type with a longer leaf path gets `E-PLT-018`. A rebind with long new string texts gets `413 E-WEB-003` (item (l)).

### Option B: A leaf path of at most 64 bytes and a body limit of 11 MiB

- Pros: the body limit is smaller.
- Cons: more types need shorter member names. Only one member name of 64 characters fits in one leaf path.

### Option C: The index form of the field names

- Pros: the largest form needs at most 5.4 MiB, with a body limit of 6 MiB. No limit on the leaf path is necessary.
- Cons: the names `v<n>` and `l<n>.min` have no hidden field. The limit mode of ADR-0039 (k) then loses the leaf path of each limit. That is a redesign of ADR-0039.

### Option D: `http.body_limit.value` as an operator setting

- Pros: an operator can raise the limit for one plant.
- Cons: no calculation supports the default value. A valid parameter can still fail at that value. The memory of the buffered bodies then has no fixed limit.

### Option E: Option A without permits

- Pros: no new layer.
- Cons: the memory of parallel bodies of 16 MiB has no limit below `container.app_memory`.

## Decision

The decision selects Option A. The owner decided the value of 128 bytes and the permits on 2026-11-22.

Length of a leaf path:

- (a) The new key `budget.leaf_path_bytes` of `reliability.md` §7.11 is 128 bytes. It limits the canonical text of a `LeafPath` (`domain-model.md` §2.1).
- (b) `DefineUdt` and `EnsureDerivedType` refuse a type with a longer leaf path with `E-PLT-018 TypeTooLarge`. The hint of the row `E-PLT-018` in `reliability.md` §1.2 adds "or shorter member names".
  - The error code is not new, because `E-PLT-018` already owns four size limits.
- (c) The check runs only where a command creates a type row. The array types of a rebind use that path through `DerivedTypes`.
  - The read of a stored type uses `RuleSet::Earlier`, so a stored type keeps a longer leaf path.
  - `TypeTree::resolve` does not apply the check, because it runs on every read.

Calculation of the body limit:

- (d) N is `budget.leaves_per_parameter` (16 384). B is `budget.value_bytes` (1 MiB). P is the encoded length of one leaf path.
  - A form body percent-encodes `[`, `]`, and `,` to 3 bytes each. The grammar of `LeafPath` holds only ASCII, so no other character of a path grows.
- (e) The pairs of one leaf need these bytes without their texts:
  1. The value pair `value` + P + `=` + text + `&` needs 7 + P bytes.
  2. The hidden pair `limits%5B16383%5D.leaf_path=` + P + `&` needs at most 29 + P bytes.
  3. The pairs of `.min` and `.max` need 23 bytes each.
- (f) A leaf that is not a string has three texts: the value, the minimum, and the maximum. Each text needs at most 40 encoded bytes, so the three need 120 bytes.
  - This limit covers the canonical texts that the server renders. The longest is an `LDT#` or `DTL#` text with 39 bytes.
  - A text that a user types can be longer. `LDATE_AND_TIME#2026-10-12-12:30:00.000000001` has 44 raw bytes and 50 encoded bytes.
  - With 50 bytes for each text, the form needs at most 16 449 536 bytes. That is still below 16 MiB.
  - Leading zeros give a typed text with no length limit. A typed form above the body limit therefore gets `413 E-WEB-003`.
- (g) The texts of the string leaves need at most 3·B encoded bytes. The fixed fields get F = 64 KiB. These are `csrf_token`, `row_version`, `type`, `name_en`, `description_en`, and the short fields.
  - `name_en` has at most 200 characters, and `description_en` has at most 4 000 characters.
  - At 4 UTF-8 bytes for each character and 3 encoded bytes for each byte, the two texts need at most 50 400 bytes.
- (h) The form needs at most F + 3·B + N·(82 + 120 + 2·P) = 6 520 832 + N·2·P bytes.
  - A path of L raw bytes with E characters `[`, `]`, and `,` encodes to L + 2·E bytes.
  - The limits of 32 levels and 3 indices for each array segment limit E.
  - For L = 128, E is at most 80. The path then has 32 segments `[0]`, and 8 of them grow to `[0,0,0]`.
  - For L = 64, E is at most 42. The path then has 21 segments `[0]`.

| L | P | Largest form | Body limit |
|---|---|---|---|
| 64 | 148 | 11 370 496 bytes (10.8 MiB) | 11 MiB |
| 128 | 288 | 15 958 016 bytes (15.2 MiB) | 16 MiB |
| Index form of Option C | none | 5 701 632 bytes (5.4 MiB) | 6 MiB |

- (i) `http.body_limit.value` changes from 4 MiB to 16 MiB in `reliability.md` §7.11, `api.md` §7.6, and `crates/recipe-web/src/settings.rs`. It stays a fixed value and does not become an operator setting.
- (j) A test builds the largest form of the table and sends it. The save succeeds. The tests that pin 4 MiB change to 16 MiB.

Stored size and raw size:

- (k) `budget.value_bytes` limits the stored size of a value (`pg_column_size`), as the `CHECK` of `data-model.md` §1.3 does.
  - The body limit of item (h) assumes a value whose raw JSON text has at most `budget.value_bytes`.
  - The two readings stand side by side. No code changes. The risks below name the result for a value that compresses well.

The rebind form:

- (l) The rebind form sends one field `new_member_values[<key>]<leaf path>` for each new member leaf (ADR-0039 (f)). ADR-0044 (b) permits at most `form.rebind_leaves` (16 384) new member leaves on one page.
  - A `ParameterKey` is a UUID of 36 bytes. One pair needs 61 + P bytes and its text: `new_member_values%5B` (20), the key (36), `%5D` (3), `=`, and `&`.
  - With P = 288 and 40 bytes for each text, the pairs need at most 16 384 · 389 = 6 373 376 bytes. With F, the sum is 6 438 912 bytes.
  - Each moved parameter can carry string texts of up to 3·B encoded bytes. The rest of 16 MiB is 10 338 304 bytes. It holds the full string texts of 3 moved parameters, but not of 4.
  - A rebind with more encoded string text gets `413 E-WEB-003`.
  - The next step of the user is a rebind with shorter texts for the new members. Then the user sets the full texts with `SetParameterValue` on each parameter. That form has the limit of item (h).

Permits of the value routes:

- (m) The new key `http.value_body_permits` of `reliability.md` §7.11 is 4. It counts the requests on the four value routes of `api.md` §7.6 that wait for, read, or hold a body at the same time.
- (n) A new layer `value_body_permit` in `crates/recipe-web/src/limits.rs` takes an owned permit for each `POST` on the four value routes. It runs inside the request timeout and outside the body deadline.
  - So the layer runs before `csrf_layer` buffers the body.
  - The body deadline starts only after the wait, so a waiting request keeps its full body deadline.
  - The request holds the permit until the handler returns its answer.
- (o) The wait for a permit lasts at most `http.body_read_timeout` (10 s). Then the layer returns `AppError::Timeout { stage: ValueBodyPermit }`.
  - `render_errors` gives `503 E-WEB-009` with `Retry-After`. The layer reads no body, and nothing writes.
  - The error code is not new, because `E-WEB-009` owns every timeout (`reliability.md` §1.1).
- (p) One request buffers its body about three times: the bytes, the pairs, and the parsed texts. A body of 16 MiB thus uses about 48 MiB.
  - The 4 permits use about 192 MiB. With the 76 MiB of the Argon2 permits, the sum is about 268 MiB.
  - `soak.memory_ceiling` is 384 MiB, and `container.app_memory` is 512 MiB.

## Consequences

### Positive

- A valid value inside the budgets of A26 and `budget.value_bytes` fits the edit form of its parameter.
- The field names and the limit mode of ADR-0039 stay.
- The buffered bodies of the value routes use at most about 192 MiB.
- No new error code and no migration are necessary.

### Negative

- A new type with a leaf path above 128 bytes gets `E-PLT-018`. The user must define shorter member names.
- A rebind with long new string texts gets `413 E-WEB-003` and needs a second step (item (l)).
- A typed text with leading zeros can push a valid form above 16 MiB.
- A client must send about 1.7 MB/s to send 16 MiB within `http.body_read_timeout`.

### Risks and mitigations

- Risk: the permit layer runs before `resolve_access` and `session_layer`. So each client takes a permit, also a client without a session.
  - Mitigation: `session_layer` refuses a missing session before the body read, and the permit returns at once.
- Risk: `csrf_layer` buffers the body before the token check and the permission check. So any authenticated role reaches the permit and holds it for the body read.
  - A client with a session can hold every permit for one body deadline. Other value requests then wait, and some get `503 E-WEB-009`.
  - Mitigation: nothing writes, and the memory stays inside the limit of item (p).
- Risk: a value that compresses well passes the `CHECK` with more than 1 MiB of raw text. Its form can exceed 16 MiB and then gets `413 E-WEB-003`.
  - Mitigation: `SetParameterValue` takes the same form fields and has the same limit. So the value cannot grow further through a form.
- Risk: a stored type has a leaf path above 128 bytes.
  - Mitigation: the type stays readable. Its edit form stays inside 16 MiB while N·P is at most 5 128 192, for example 16 384 leaves with P = 313.

## Related

- ADR-0039 (a), (f), (g), (k), amended by this ADR in (g): the body limit of the value routes
- ADR-0044 (b), (d): the limit of the rebind forms
- ADR-0058 (g), (j): the request timeout and the body deadline
- `reliability.md` §1.1, §1.2, §4.4, §7.4, §7.8, §7.11
- `api.md` §2, §7.6
- `domain-model.md` §2.1
- `data-model.md` §1.3
- `overview.md` §7: R13, R30, R38. `overview.md` §9: A26
