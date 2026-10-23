## Context

Step P3.5 adds the export document v1. The use cases `ExportApprovedVersion` and `ExportVersionByNumber` exist in `recipe-app`. They lock the version `FOR SHARE`, rebuild the hashed body, compare its hash, and commit `VersionExported`. The crate `recipe-export` holds only `lib.rs`. `recipe-web` has no export route and no export link.

Normative sources:

- `api.md` sections 1, 2 (intro), 2.5, 2.7, 3, 5, 6, 8, 9,
- `reliability.md` sections 1.1, 1.2, 2, 4.4, 7.4, 7.8, 7.11,
- `domain-model.md` sections 2.1, 5, `data-model.md` sections 6 (row `ExportVersion`) and 10,
- `design-system.md` sections 7, 8, 10,
- ADR-0019, ADR-0049 (i), ADR-0052, ADR-0053, ADR-0054, ADR-0055, ADR-0056.

The binding decisions of the archived designs of steps P3.1 to P3.4 stay valid. The research of step A1 is in `target/p35-agents/R1.md` to `R4.md` and `VQ.md`. The plan review is in `PR1.md` to `PR4.md`, and its triage is in `plan-review-triage.md` of the same directory. These files are not normative.

## Goals / Non-Goals

Goals:

- The schema file, the document writer, and the golden tests of ADR-0019.
- The two routes of `api.md` section 2.7 with the answers of ADR-0053 and ADR-0054 and the guard of ADR-0055.
- The export link of ADR-0052.
- A flow that writes the document bytes before the commit of `VersionExported`.
- The how-to guide `docs/user/check-export-document.md`.

Non-Goals:

- The counter of `E-EXP-002` for the job `alerts` (step P4.1, D1).
- `serve`, Compose, and the health listener (step P4.1).
- The Playwright tests of the rows "browser" of D11 (step P4.2, ADR-0048 (f)).
- A coded `404` for an unknown path or method (step P4.2, ADR-0053 (g)).
- A migration, a grant, an index, or a new error code.
- The hover pair of `a.btn` in `design-system.md` section 4 and in the contrast test (step P4.2 follow-up).

## Decisions

### D1. Answers of the user (2026-10-22 and 2026-10-23)

Every answer of 2026-10-22 took the recommended option. The two answers of 2026-10-23 decide findings of the step-6 reviews. A verifier (T1-O, read only) checked the recommendations before the questions.

| Question | Answer | Record |
|---|---|---|
| a. UI | Version page only. `Approved`: link to `/approved` for each role with `VersionExport`. `Retired`: link to `/versions/{n}` for a role with `VersionExportHistory`, with a notice. A plain link outside every boosted container, with `hx-boost="false"`, without `download`, without `target`, without an icon | ADR-0052 |
| b. JSON error | Every answer of 400 or more on `/export/v1/` is JSON. `message` in the UI language. No `HX-*` header. `recipe-export` writes the body | ADR-0053 (a) to (f) |
| b2. Session | `E-WEB-004` gives `303 /login`. `E-IDN-007` gives `303 /me/password` | ADR-0053 (d) |
| c. Validator | `jsonschema` 0.60.0, dev-dependency of `recipe-export` only, `default-features = false`, feature `arbitrary-precision`. Format checks on in the tests | D4 |
| d. Document | One how-to guide `docs/user/check-export-document.md` with links and no field table | D10 |
| e. Golden | The writer orders a struct value by `types[].members[]`. The golden document is the example of `api.md` 6.2. `api.md` does not change | D3, D5 |
| M2. Schema | Closed: `additionalProperties: false` on each object with fixed fields. A new field is v2 | D4 |
| f. Counter | Step P4.1 with the job `alerts`. The flow of this change does not count `E-EXP-002` (gap against `data-model.md` 6 and `reliability.md` 3.7) | P4.1 follow-up |
| g. `HEAD` | Only `GET`. `HEAD` gets the answer of an unlisted method | ADR-0054 |
| h. Writer | A port in `recipe-app`. The flow writes the bytes before the commit. `E-EXP-004` and `E-WEB-011` roll back | D6 |
| D3 of the plan | The hash does not change after a retirement. `loadable` becomes `false` (user prompt, item 5: `api.md` wins over the plan) | D11 |
| U1. Guard (plan review, session 2) | One guard before the use case on every path under `/export/v1/`. A present `Sec-Fetch-Site` other than `same-origin` or `none`, or `HX-Request: true`, gives `403 E-WEB-007` as JSON and one `csrf_rejected` event. A missing `Sec-Fetch-Site` passes. No new code | ADR-0055, D7 (f) |
| SR-1. Parallel exports (step-6 review, 2026-10-23) | Fix now. At most `export.permits` exports run at the same time. The wait for a permit is inside `timeout.export`. No new code | ADR-0056, D15 |
| AD-1. Type reference (step-6 review, 2026-10-23) | Fix now. A type reference in the module `export` of `recipe-domain` is a domain type, not text. The hash and the golden files do not change | `.claude/rules/rust.md`, D3 (c) |

VQ named these points as decisions of the orchestrator, with no question for the user:

- the RFC 3339 form of D3 (b),
- the pretty output of D5 (a),
- the file name of D5 (b),
- the writer of the JSON error body in `recipe-export` (D5 (c)),
- the writer inside the rebuild task (D6).

### D2. Module layout

`recipe-domain` (module `export`):

- `parameters.rs`: the value of an entry keeps the member order of its struct type (D3 (a)).
- `body.rs`, `types.rs`, `parameters.rs`: read-only accessors for the writer (D3 (c)).
- `facts.rs` (new): `DocumentFacts`, `ExportStatus`, `SCHEMA_VERSION`, and the time text of D3 (b).
- No feature `test-support` and no fixture module. `test_support.rs` stays `#[cfg(test)]`. The golden tests of `recipe-export` build their bodies (D14).

`recipe-app`:

- `src/ports/document_writer.rs` (new): the port `DocumentWriter` (D6).
- `src/export/`: the flow of D6, the output `ExportedVersion` with the document bytes, the constructors with the writer.
- `src/memory/document_writer.rs` (new, feature `memory`): `BodyWriter`, a test double that writes the hashed body with `serde_json`.
- `src/memory/fixtures/`: the user rows of the approvers (D14).

`recipe-export`:

- `schema/export-v1.schema.json` (new, D4).
- `src/document.rs` (new): `write_document` (D5 (a)).
- `src/file_name.rs` (new): `file_name` (D5 (b)).
- `src/error_body.rs` (new): `error_body` (D5 (c)).
- `tests/schema.rs`, `tests/golden.rs`, `tests/golden/` (new). `tests/golden.rs` builds the two bodies through the public constructors of `recipe-domain`.

`recipe-web`:

- `src/error/layer.rs`, `src/error/answer.rs` (and a new `src/error/json.rs` when `answer.rs` comes near 800 lines): the JSON answer of ADR-0053.
- `src/pages/export.rs` (new): the two handlers, the headers, and the adapter `ExportWriter` of the port `DocumentWriter`.
- `src/routes.rs`: the two rows and the `HEAD` rule of ADR-0054.
- `src/pages/sweep_tests.rs`, `src/pages/xss_tests.rs`: the page sweeps skip the export rows. A new sweep checks the `/export/` anchors (D11 T14).
- `src/pages/versions/actions.rs`, `templates/version.html`: the link of ADR-0052.
- `src/export_guard.rs` (new): the guard of ADR-0055 (D7 (f)). `assemble` in `src/routes.rs` puts it as a layer on each row under the prefix of D7 (b).

`recipe-db`: tests only (D11).

### D3. Domain: member order, views, facts

- (a) `ParameterEntry` keeps the value as a tree that writes the members of each struct in the order of `members[]` of its `TypeNode`. An array keeps its element order. The build walk uses an explicit stack (`reliability.md` section 2.2). The ordered value serializes by recursion of serde. The type depth of the domain bounds that recursion. Keys of a stored struct value that `members[]` does not name come after the members, in byte order. The canonical form of RFC 8785 sorts the keys, so `content_sha256` does not change. The stored test vectors of `export/hash/tests.rs` prove it. X1 tests a struct in an array and a 2-D array of a struct with a `TypeNode` that the test builds by hand.
- (b) `exported_at` and `version.approved_at` are UTC text `YYYY-MM-DDTHH:MM:SSZ` with whole seconds, as the example of `api.md` 6.2. The function converts the instant to UTC with `checked_to_offset`, truncates it to the second, and writes the digits by hand. `recipe-domain` gets no `time/formatting` feature. A year outside 0000 to 9999 gives `ExportError::Serialize` (`E-EXP-004`). The function lives in the module `export` of `recipe-domain`.
- (c) `HashedBody` gets read-only accessors: the machine code, the platform, the reference number, the version number, the type entries, and the parameter entries. A parameter entry gets accessors for each field. It gives its key as `ParameterKey`, its unit path as `UnitPath`, its tag as `ParameterTag`, and its S7 address as `Option<S7Address>`. The reason is the rule of `.claude/rules/rust.md`: no bare `String` for a domain identifier. A type reference is a domain type of a type reference, not text (user decision AD-1). This applies to `ParameterEntry::type_id`, `TypeEntry.id`, `TypeEntry.element_type_id`, and `Member.type_id`. Its text equals the old text, so the hash and the golden files do not change. The value objects `UnitPath`, `ParameterTag`, `S7Address`, and `ParameterKey` have no `Serialize`. The type `TypeReference` has its own `Serialize`, which writes the same text as its `Display`. The writer reuses the `Serialize` of the entry or calls `to_string`. The type entry, the limit entry, and the ordered value stay serializable, so the writer reuses their `Serialize`. No accessor gives a mutable reference. X2 adds doc comments to `TypeEntry` and `LimitEntry`: their field order is part of the contract of `api.md` 6.1.
- (d) `DocumentFacts` holds the fields outside the hash: `exported_at`, `exported_by` (`Login`), `status` (`ExportStatus`), `approved_at`, `approved_by` (`Login`), the machine name, the recipe name, `content_sha256`, and `names`. `names` is a map from `ParameterKey` to `Text` and gives `name_en`. `DocumentFacts` is a plain struct with public fields, like `ExportHeader`. `loadable` follows from `status`.
- (e) `SCHEMA_VERSION` is the one constant `"1"`. The flow uses it in the audit entry, and the writer uses it in the document.
- (f) `ExportStatus` is a domain enum with only `Approved` and `Retired`. It serializes as `Approved` and `Retired`. The flow converts `VersionStatus` once, after the state check.

### D4. Schema file

- `crates/recipe-export/schema/export-v1.schema.json`, draft 2020-12, `$id` with `schema_version` `"1"`.
- Each object with fixed fields has `additionalProperties: false` and lists every field of `api.md` 6.1 in `required`. The objects of `value` stay open, because their keys are member names.
- `$defs/type_ref`: a type reference is a primitive name of an `enum` (the primitives without a length) or a UUID. `parameters[].type_id`, `types[].element_type_id`, and `members[].type_id` use it.
- `$defs/value`: a recursive schema with `anyOf` of a string, a boolean, an array of `value`, and an object with `additionalProperties: {"$ref": "#/$defs/value"}`. A JSON number and a null fail.
- Each `types[]` item declares all eight fields in its `properties`, so `additionalProperties: false` accepts them. `if`/`then` on `kind` adds `required` for the fields of that kind. It forbids the fields of the other kinds with `not: {required: [...]}`.
- Rules per kind of `types[]`: `name` is null only for `Array`. `string_length` is an integer only for `Primitive`. `revision` is `1` for every kind except `Struct`. `members` has at least one item.
- Other rules:
  - `schema_version` is the `const` `"1"`,
  - `version.status` is an `enum` of `Approved` and `Retired`,
  - `loadable` is `false` when `version.status` is `Retired`,
  - `s7_address` is a string on `S7_300` and null on `CodesysIec` and `TwinCat3` (A31).
- Patterns:
  - a UUID for each identifier,
  - 64 lower-case hex characters for `content_sha256`,
  - the time text of D3 (b),
  - the canonical decimal text of `reliability.md` section 4.4 for `min`, `max`, and `max_length`: `^(0|-?[1-9][0-9]*(\.[0-9]*[1-9])?|-?0\.[0-9]*[1-9])$`,
  - the grammars of `domain-model.md` section 2 for `reference_number`, `unit_path`, `tag`, and `leaf_path`.
- `dimensions` has 1 to 3 items (A38). `plc_platform` and `kind` use `enum`.
- The tests compile the schema with `jsonschema::draft202012::options()` and turn on the format checks (`should_validate_formats(true)`).
- Dependency: `jsonschema = { version = "0.60.0", default-features = false, features = ["arbitrary-precision"] }` in `[workspace.dependencies]` and in `[dev-dependencies]` of `recipe-export` only. No HTTP, TLS, file, or ICU code. `overview.md` section 5 does not change, as for `proptest` and `testcontainers`.
- `cargo deny --locked check` runs after the change. When it rejects the licence `MIT-0` of `borrow-or-share`, the agent stops and reports. The user decides about `deny.toml` and its commit.

### D5. Writer, file name, error body

- (a) `write_document(&HashedBody, &DocumentFacts) -> Result<Vec<u8>, ExportError>` builds typed structs in the field order of `api.md` 6.1 and writes them with `serde_json::to_vec_pretty` (two spaces, no newline at the end). It never builds an object through `serde_json::Value`. An error gives `ExportError::Serialize`. A parameter key without an entry in `DocumentFacts.names` also gives `ExportError::Serialize` (`E-EXP-004`).
- (b) `file_name(&ReferenceNumber, u32, ExportStatus) -> String` gives `recipe_{ref}_v{n}.json` or `RETIRED_recipe_{ref}_v{n}.json` (`api.md` 6.3). The grammar of `ReferenceNumber` permits only `A-Z`, `0-9`, `_`, and `-`, so the name needs no escape.
- (c) `error_body(code: ErrorCode, message: &str, request_id: RequestId) -> Vec<u8>` writes the compact JSON object with the three fields in this order (ADR-0053 (a)). When `serde_json` fails, the function writes a fixed body with the same code, an empty `message`, and the same `request_id`. It never panics. The request id is the newtype `recipe_domain::audit::RequestId` (`.claude/rules/rust.md`), so a caller cannot swap it with the message. Its text is a UUID and needs no escape (user decision after the verifier of wave 2).
- (d) `recipe-export` gets the dependencies `serde` and `serde_json` that ADR-0019 names. It returns `ExportError` and has no error type of its own.

### D6. Flow of `ExportVersion`

- (a) `recipe-app` defines the port `DocumentWriter` with one method `write(&self, body: &HashedBody, facts: &DocumentFacts) -> Result<Vec<u8>, ExportError>`. The trait needs `Send + Sync`.
- (b) `ExportApprovedVersion::new` and `ExportVersionByNumber::new` take the context and an `Arc<dyn DocumentWriter>`. `recipe-web` passes its adapter over `recipe_export::write_document`. `recipe-server` never names `recipe-export` (`overview.md` section 5).
- (c) The order inside `timeout.export`, after the wait for a permit of D15:
  1. Read the recipe and lock the version `FOR SHARE`.
  2. Read the recipe again and check the state.
  3. Read `version_detail` for the login of the approver.
  4. Read the parameters, the units, and the types.
  5. One `spawn_blocking` task builds the body, computes the hash, and compares it with the stored hash. Then it calls the writer.
- The commit of `VersionExported` runs after the limit, as before (ADR-0029).
- (d) `exported_at` is one `ctx.clock.now()` read before the task. `DocumentFacts.names` comes from the parameters of the version.
- (e) A writer error gives `E-EXP-004`, and a failed task gives `E-WEB-011`. Both drop the transaction, so the audit log gets no entry (`data-model.md` 6, `reliability.md` 6: no partial write). `rebuild_export` maps the `JoinError` to `E-WEB-011`. T7 fails the task with a writer double that panics.
- (f) `ExportedVersion` holds the document bytes, the reference number, the version number, the status (`ExportStatus`), and `content_sha256`. The web layer needs no other field.
- (g) A missing `approved_at` or a missing approver login of an `Approved` or `Retired` version is corrupt data. It gives `StorageError::invariant` (`E-DB-008`). A memory test covers it.
- (h) A build error of the body gives `E-EXP-002` on an export route. Examples are `E-RCP-020` and a unit or a type that the snapshot does not hold. The server cannot rebuild the stored hash. The error list of `data-model.md` 6 has no `E-RCP-020`.

### D7. Routes and answers in `recipe-web`

- (a) Two rows of the route table: `GET /export/v1/recipes/{ref}/approved` with `ExportApprovedVersion` and `GET /export/v1/recipes/{ref}/versions/{n}` with `ExportVersionByNumber`. The handlers parse the path values with `path_reference` (`pages/recipes.rs`) and `path_version` (`pages/versions.rs`). Both take the rejection of axum as a value. A bad or undecodable value, for example `%FF`, gives `404 E-WEB-005` as JSON.
- (b) One constant names the prefix `/export/v1/`. The error layer reads it before the handler runs (ADR-0053). The access table reads it for the `HEAD` rule (ADR-0054). A `HEAD` request on that prefix gets the answer of an unlisted method.
- (c) A success answers `200` with `Content-Type: application/json`, `Content-Disposition: attachment; filename="<file_name>"`, and `Cache-Control: no-store`. The handler builds each header value with `HeaderValue::from_static` or `HeaderValue::from_str` and never passes a `String` value to axum (R3 constraint 26). An `Err` of `HeaderValue::from_str` gives `500 E-WEB-010` as JSON, with no panic. The grammar of `ReferenceNumber` makes this branch unreachable.
- (d) The JSON answer of ADR-0053 uses `error_body` of D5 (c). The message is the text of `error-<code>` with the arguments of the error, in the UI language of the request. The arguments of `E-EXP-001` to `E-EXP-003` are `reference` and `number`.
- (e) The CSRF layer exempts `GET` (`api.md` section 7.3). The read-only gate lets both operations pass (A24).
- (f) The guard of ADR-0055 is a layer of each route row under the prefix of D7 (b). `assemble` adds it with the state of the router, because the guard writes its event through `SecurityEventSink`. The route rows have no state (user decision after the verifier of wave 4). It runs after the session layer and the gates, and before the use case. So `E-WEB-004` and `E-IDN-007` keep their redirects (ADR-0053 (d)). A `Sec-Fetch-Site` header other than `same-origin` or `none`, or `HX-Request: true`, gives `403 E-WEB-007` as JSON and one `csrf_rejected` event through `SecurityEventSink`. A request without `Sec-Fetch-Site` passes. W4 owns the guard (task 7.5).

### D8. Export link

ADR-0052 gives the rules. `version_actions` adds an optional export link with the `href`, the text, and a flag for the notice of a `Retired` version. It uses `Operation::ExportApprovedVersion` or `Operation::ExportVersionByNumber` with `requirement().allows(role)`, as the other actions do. The template puts the link into its own `<p class="actions">` after the header `dl`. For a `Retired` version, the notice stands in `<div id="export-notice">` directly before that `p`, as `components/danger.html` does. A `div` never stands inside a `p`. The notice uses the info notice of `components/alert.html`, and `alert.html` does not change. The link of an `Approved` version has no `aria-describedby`. No new CSS rule is necessary.

### D9. Texts

The keys go into the main bundles `i18n/en.ftl` and `i18n/pl.ftl`. The approval files hold only the keys of design D8 of step P3.4. A test pins that list (orchestrator decision after the stop of H1). `error-E-EXP-004` stays in the approval files. The English message of each code equals its row of `reliability.md` section 1.2 without the sentence "Reference: {request_id}.".

| Key | en | pl |
|---|---|---|
| `error-E-EXP-001` | Recipe { $reference } has no approved version. | Receptura { $reference } nie ma wersji zatwierdzonej. |
| `error-E-EXP-002` | Version { $number } of recipe { $reference } does not match its stored hash. The export is refused. Contact the administrator. | Wersja { $number } receptury { $reference } nie zgadza się z zapisanym skrótem. Eksport jest odrzucony. Skontaktuj się z administratorem. |
| `error-E-EXP-003` | Version { $number } of recipe { $reference } is a draft or was discarded. Only approved and retired versions can be exported. | Wersja { $number } receptury { $reference } jest szkicem albo została odrzucona. Eksportować można tylko wersje zatwierdzone i wycofane. |
| `version-export-link` | Export version { $number } as JSON | Eksportuj wersję { $number } do pliku JSON |
| `version-export-retired-notice` | The file name starts with RETIRED_. The file marks this version as not loadable into a PLC. | Nazwa pliku zaczyna się od RETIRED_. Plik oznacza, że tej wersji nie można załadować do PLC. |

`error-E-EXP-004` exists. The other codes of the export routes use their existing keys.

H1 adds only `error-E-EXP-001` to `error-E-EXP-003`. W3 adds `version-export-link` and `version-export-retired-notice` together with the template that uses them. The usage test of the approval keys then stays green after each wave.

### D10. How-to guide

`docs/user/check-export-document.md` is one how-to guide (Diátaxis) with the title "Check an export document before a PLC load":

1. The purpose and the reader.
2. The sources: the schema file in the repository, `api.md` sections 2.7, 5.3, 6.1, 6.3, and `domain-model.md` section 2.1.
3. The prerequisites: a JSON Schema validator of draft 2020-12, an RFC 8785 library, SHA-256, and a JSON parser that keeps strings and integers exact.
4. A numbered procedure:
   1. Check the document against the schema.
   2. Check `schema_version` and `loadable`. Compare `version.number` with the expected version.
   3. Build the hashed body from the fields with "In hash: yes". Keep the array order and each `null`.
   4. Write the canonical form and compute the hash.
   5. Compare the hash with `content_sha256`.
5. A link to the golden files of `recipe-export` as the test vector.
6. What to report when a step fails.

The page holds no field table and no rule that `api.md` does not state. `ste-lint.py` reports 0 hard violations on it (ADR-0023).

### D11. Test plan of the required states

| Id | State | Level, crate | Task |
|---|---|---|---|
| T1 | The struct value keeps the member order, and the stored hash vectors stay equal. A struct in an array and a 2-D array of a struct, from a `TypeNode` that the test builds by hand. A key outside `members[]` comes after the members | unit, `recipe-domain` | 3.1 |
| T2 | The accessors give the fields of the body. The time text of D3 (b) gets three inputs: the offset `+02:00`, a fraction of a second, and a year outside the range | unit, `recipe-domain` | 3.2 |
| T3 | The valid document is the example of `api.md` 6.2 with the real hash. It passes the schema. These field cases each fail: an extra field, a missing required field of each kind, `members` on a primitive, and a null `element_type_id`. These value cases each fail: a JSON number or null in `value`, a bad decimal text, and a bad type reference. These rule cases each fail: `schema_version` `"2"`, `loadable: true` with `Retired`, a bad hash, and a bad time | integration, `recipe-export` | 4.1 |
| T4 | `file_name` with and without `RETIRED_`. `error_body` with the field order | unit, `recipe-export` | 4.2 |
| T5 | The golden document of `api.md` 6.2 and its hash, byte for byte. The golden document of a `Retired` version with an `Array` type. Both pass the schema. A body of `TwinCat3` writes `s7_address` as `null` and passes the schema. The test reads each golden file and strips each `\r` before the byte comparison. The JSON block of `api.md` 6.2 equals the golden document, except `content_sha256`. That comparison ignores white space outside JSON strings only | integration, `recipe-export` | 4.3 |
| T6 | The writer gets the facts. Two exports give the same hash. After a retirement the hash stays and `loadable` is `false`. The entry holds the version id, `"1"`, and the hash | unit, `recipe-app` | 5.1 |
| T7 | A writer error gives `E-EXP-004`. A writer double that panics fails the task, and the flow gives `E-WEB-011`. Neither writes an entry. A missing `approved_at` or approver login gives `E-DB-008`. A build error of the body gives `E-EXP-002` | unit, `recipe-app` | 5.1 |
| T8 | A changed value under the stored hash gives `E-EXP-002` and no entry on PostgreSQL. The hash stays after a retirement. A writer error writes no entry | integration, `recipe-db` | 6.1 |
| T9 | The link per role and state: `Draft`, discarded draft, `Approved`, `Retired`, deleted recipe, and read-only mode. The test compares the exact text from `</dl>` to the end of the export `p`. `hx-boost="false"`. No `download`, no `target`. The notice and `aria-describedby` for `Retired`. No `aria-describedby` for `Approved` | HTTP, `recipe-web` | 7.1 |
| T10 | Each error family on an export path is JSON in this field order, with the message in `pl` and `en`. No `HX-*` header. `E-WEB-004` and `E-IDN-007` redirect. `E-WEB-004` with `HX-Request: true` gives `303` to `/login` (ADR-0053 (d)). `Retry-After` stays. The JSON answers and the `303` have `Cache-Control: no-store`, `X-Content-Type-Options: nosniff`, and the CSP. A JSON answer has `Content-Type: application/json` | HTTP, `recipe-web` | 7.2 |
| T11 | Both routes: `200`, the three headers, `RETIRED_`, the document, the entry. `nosniff` and the CSP on the success. `HEAD` gives no entry. `HEAD` on a page route, for example `/recipes`, still uses the `GET` row (ADR-0054 (c)). Read-only mode exports and writes the entry (A24). `E-EXP-001`, `E-EXP-002`, `E-EXP-003`, `E-RCP-012`, `E-WEB-005` as JSON. `%FF` as the reference or the version number gives `404 E-WEB-005` as JSON | HTTP, `recipe-web` | 7.3 |
| T12 | Each export route for each of the six principals: status, code, and the `forbidden` event | HTTP, `recipe-web` | 7.4 |
| T13 | The guard of ADR-0055 on both routes: `Sec-Fetch-Site` `cross-site` and `same-site` give `403 E-WEB-007` as JSON, one `csrf_rejected` event, and no `VersionExported` entry. `HX-Request: true` gives the same. `none` and an absent header pass | HTTP, `recipe-web` | 7.5 |
| T14 | Every page: each anchor with an `href` under `/export/` has `hx-boost="false"` and no boosted ancestor | HTTP, `recipe-web` | 7.3 |
| T15 | The gate of D15. While `export.permits` exports hold a permit, a second export waits. A wait past `timeout.export` gives `E-WEB-009` and no `VersionExported` entry. A task that runs after the limit keeps its permit. The default of `export.permits` is 2 | unit, `recipe-app` | 5.2 |
| T16 | The type reference of a parameter entry, a type entry, and a member is a domain type of a type reference. Its text equals the old text. The stored hash vectors and both golden files stay equal | unit, `recipe-domain`. Integration, `recipe-export` | 3.3 |

Rows for the browser (step P4.2, ADR-0048 (f)):

| Id | Width | State | Expected |
|---|---|---|---|
| B1 | 1280 | Administrator retires `Approved` version 2 with the boosted retire form, so `#content` shows the `Retired` version 2. Then a click on the link. The version lists hold no boosted link | The browser saves `RETIRED_recipe_R-100_v2.json`. `#content` gets no swap, and the export request has no `HX-Request` header. `link.closest('[hx-boost="true"]')` is null. The page stays, and the focus stays on the link |
| B2 | 1280 | Operator, `Approved` version 3, click | The browser saves `recipe_R-100_v3.json`. The SHA-256 of the hashed body of the file equals `content_sha256` |
| B3 | 320 | Version page of an `Approved` version, and of a `Retired` version with the notice | `scrollWidth` equals `clientWidth`. The link is at least 44 by 44 CSS px. The gap to the machine link and the gap to the first link of the row of links are each at least 8 px |
| B4 | 1280 | The approved version is retired in another tab, then a click on `/approved`. Also `/versions/2` of a deleted recipe | The tab shows the JSON body with `E-EXP-001` and the `request_id`. The body of `E-RCP-012` shows the Polish letters. Back returns to the version page |
| B5 | 1280 | Keyboard only | The Tab order is the machine link, the export link, and the first link of the row of links. The computed `box-shadow` of the focused link shows the focus ring. `toHaveAccessibleName` gives the `pl` link text. `toHaveAccessibleDescription` gives the notice text of a `Retired` version |
| B6 | 1280 | The page of `Approved` version 3 is open. A second browser context approves version 4. Then a click on the link | The browser saves `recipe_R-100_v4.json`, and `version.number` is 4 |
| B7 | 1280 | Forced-colors mode (`forcedColors: 'active'`), keyboard focus on the link | The outline of the focused link is visible |

### D12. Known changes of test expectations

The orchestrator adds a row for each further change that an agent reports. Paths start at the crate of the row.

| File (crate) | Old expectation | New expectation | Reason | Agent |
|---|---|---|---|---|
| `src/pages/sweep_tests.rs:85-92,132-148,181-197` (`recipe-web`) | `get_page_patterns()` takes every `GET` row with an operation and expects an HTML page | The function leaves out the rows under the prefix constant of D7 (b) | An export row answers JSON (ADR-0053), and `pages()` has no request for it | W2 |
| `src/pages/xss_tests.rs:74-91` (`recipe-web`) | The probes reach every `GET` row with an operation and expect `200` or `422` | The probes skip the export rows through the filter of `sweep_tests.rs` | Same reason. T10 and T11 check the headers of the export answers | W2 |
| `src/error/export_json_tests.rs` (`recipe-web`) | An HTMX request on a test route under the prefix gets the JSON answer of its error family | The guard answers it with `403 E-WEB-007` as JSON before the handler. `E-WEB-004` and `E-IDN-007` keep their redirects | ADR-0055 (a), (c), (e): the guard covers each route row under the prefix after the session layer and the gates | W4 |
| `src/routes/tests.rs:255-333` (`recipe-web`) | The table holds the rows of the matrix, the static rows, and the rows of nine areas | The table also holds the two rows of the area `export`. The coverage tests list them. Task 7.4 adds their rows for the six principals in its own file | The two routes of `api.md` section 2.7 | W2 |
| `src/pages/versions/layout_tests.rs:85-94` (`recipe-web`) | The boosted row of links follows `</dl>` | The export row follows `</dl>`, with the notice before it for `Retired`. Then the boosted row | ADR-0052 (c), (e) | W3 |
| `src/pages/versions/actions_tests.rs:38-43,130-148,165-181` (`recipe-web`) | The action area holds no export link | The action area holds the export link of the role and the state | ADR-0052 (b), (g) | W3 |
| `src/pages/versions/compare_tests.rs:29-33,124-125` (`recipe-web`) | The boosted row of links follows `</dl>` | The export row stands between `</dl>` and the boosted row | ADR-0052 (c) | W3 |
| `src/pages/recipe_sweep_tests.rs:48,285-304` (`recipe-web`) | Only `hx-boost="true"` is permitted | Also the literal `hx-boost="false"` of the export link, and only there | ADR-0052 (c) | W3 |
| `src/export/tests.rs:136-140` (`recipe-app`) | The test reads `body`, `machine_name`, `recipe_name`, `approved_by`, and `approved_at` of `ExportedVersion` | The writer double records `DocumentFacts` and the body, and the test asserts the same facts there | D6 (f) takes out `body`, `machine_name`, `recipe_name`, `approved_by`, `approved_at`, `exported_at`, and `exported_by` | A1 |
| `src/export/tests.rs:198,220,313` (`recipe-app`) | The test compares `body` and `recipe_name` of two exports | The test compares the recorded body and the recorded recipe name | D6 (f) | A1 |
| `src/export/tests.rs:131,196` (`recipe-app`) | `status` is a `VersionStatus` | `status` is an `ExportStatus` | D3 (f), D6 (f) | A1 |
| `src/export/body/tests.rs:76` (`recipe-domain`) | `speed.type_id()` equals `"INT"` | `speed.type_id()` equals `TypeReference::Primitive(Primitive::Int)` | D3 (c): the accessor gives a domain type of a type reference | X3 |

### D13. Waves

Classes: T1-O `opus` max, T1-C `opus` high, T2 `opus` xhigh, T3 `sonnet` high, T4 `haiku` medium. No agent uses `fable`. An agent that writes or fixes only tests is T3.

| Wave | Agent | Class | Model | Effort | Task | File set |
|---|---|---|---|---|---|---|
| 1 | X1 | T1-C | opus | high | 3.1 `[hard]` | `recipe-domain`: `src/export/parameters.rs`, `src/export/parameters/tests.rs`. Read only: `src/export/hash/tests.rs` |
| 1 | E1 | T1-C | opus | high | 4.1 `[hard]` | `Cargo.toml` (the line of `jsonschema` in `[workspace.dependencies]` only), `Cargo.lock`, `crates/recipe-export/Cargo.toml` (`[dev-dependencies]` only), `crates/recipe-export/schema/export-v1.schema.json`, `crates/recipe-export/tests/schema.rs` |
| 1 | H1 | T4 | haiku | medium | 2.1 `[mechanical]` | `recipe-web`: `i18n/en.ftl`, `i18n/pl.ftl` (the keys `error-E-EXP-001` to `error-E-EXP-003` only), `src/i18n/export_texts_tests.rs` (new), its `mod` line in `src/i18n.rs`, the list `codes_with_a_message` in `src/i18n/tests.rs` (the codes `E-EXP-001` to `E-EXP-003` only). Also the undo of its own first edit of `i18n/en-approval.ftl` and `i18n/pl-approval.ftl` |
| 2 | X2 | T1-C | opus | high | 3.2 `[hard]` | `recipe-domain`: `src/export/mod.rs`, `body.rs`, `types.rs` and `parameters.rs` (accessors and doc comments only), `facts.rs` (new), the tests of `src/export/` |
| 2 | E3 | T3 | sonnet | high | 4.2 | `recipe-export`: `Cargo.toml` (`[dependencies]` only), `src/lib.rs`, `src/file_name.rs`, `src/error_body.rs` (new), `Cargo.lock` |
| 2 | W3 | T3 | sonnet | high | 7.1 | `recipe-web`: `src/pages/versions/actions.rs`, `src/pages/versions/page.rs`, `templates/version.html`, `i18n/en.ftl` and `i18n/pl.ftl` (the two link keys only, next to the `version-*` keys), a new test file `src/pages/versions/export_link_tests.rs` with its `mod` line in `src/pages/versions.rs`. The changed tests of D12: `src/pages/versions/layout_tests.rs`, `actions_tests.rs`, `compare_tests.rs`, and `src/pages/recipe_sweep_tests.rs` (the allow rule for the literal `hx-boost="false"` of the export link only) |
| 3 | A1 | T1-C | opus | high | 5.1 `[hard]` | `recipe-app`: `src/ports/document_writer.rs` (new), `src/ports/mod.rs`, `src/export/`, `src/memory/document_writer.rs` (new), `src/memory/mod.rs`, `src/lib.rs`, `src/memory/fixtures/` (the seed files, for the user rows of the approvers). The callers of the two use cases: `src/auth/first_statement.rs`, `src/auth/matrix.rs`, `src/memory/fixtures/scenarios.rs`, `src/audit/coverage.rs` (no new line). `recipe-db`: `tests/storage/atomic/recipe.rs`, `tests/storage/scenario/scene.rs` (the constructor calls only). `recipe-web`: `examples/dev_server.rs` only when it builds the use cases |
| 3 | E2 | T1-C | opus | high | 4.3 `[hard]` | `recipe-export`: `src/lib.rs`, `src/document.rs` (new), `tests/golden.rs` (new), `tests/golden/` (new), `Cargo.toml` (`[dev-dependencies]` only: `time` and `rust_decimal` with `workspace = true`, no new package in `Cargo.lock`. E3b adds `uuid` in wave 2) |
| 3 | W1 | T1-C | opus | high | 7.2 `[hard]` | `recipe-web`: `src/error/layer.rs`, `src/error/answer.rs`, `src/error/json.rs` (new, if needed), `src/error.rs` (the `mod` line only), a new test file `src/error/export_json_tests.rs`. The constant of the prefix goes into a new file `src/export_path.rs` with its `mod` line in `src/lib.rs` |
| 4 | W2 | T1-C | opus | high | 7.3 `[hard]` | `recipe-web`: `src/pages/export.rs` (new), its tests in `src/pages/export/` (new), `src/pages.rs`, `src/routes.rs` (the rows and the `HEAD` rule), `src/routes/tests.rs`, `src/routes/matrix.rs`, `src/pages/sweep_tests.rs`, `src/pages/xss_tests.rs`, `src/test_support.rs` and `src/test_support/recipes.rs` (new seeds and helpers only) |
| 5 | W4 | T1-C | opus | high | 7.5 `[hard]` | `recipe-web`: a new `src/export_guard.rs` with its `mod` line in `src/lib.rs`, and a new test file `src/export_guard/tests.rs` with its `mod` line. Also the layer call in `assemble` of `src/routes.rs` and the HTMX expectations of `src/error/export_json_tests.rs` (D12). It uses the prefix constant of W1. In `src/csrf.rs`, it makes `rejection_event`, `reject`, `SEC_FETCH_SITE`, and `SAME_ORIGIN` `pub(crate)` and changes no behavior (user decision before wave 5) |
| 4 | D1 | T3 | sonnet | high | 6.1 | `recipe-db`: a new test file `tests/storage/atomic/export.rs`, its `mod` line, `tests/storage/support/` (new helpers only), `tests/storage/atomic/recipe.rs` (the visibility of the shared helpers only) |
| 4 | D2 | T3 | sonnet | high | 1.5 (no cargo) | `docs/user/check-export-document.md` (new) |
| 4 | C1 | T3 | sonnet | high | 1.4 (no cargo) | `CHANGELOG.md` |
| 5 | H2 | T3 | sonnet | high | 7.4 | `recipe-web`: a new test file `src/pages/export/access_tests.rs`, its `mod` line in `src/pages/export.rs`. It builds a local world with `seed_approval_recipes` and reuses the `pub(crate)` items of `src/routes/matrix.rs` |
| 6 | X3 | T1-C | opus | high | 3.3 `[hard]` | `recipe-domain`: `src/export/` (the type reference of D3 (c) and its tests). `recipe-export`: `src/document.rs` |
| 6 | P1 | T1-C | opus | high | 5.2 `[hard]` | `recipe-app`: `src/blocking.rs`, `src/blocking/tests.rs`, `src/context.rs`, `src/context/tests.rs`, `src/export/flow.rs`, `src/export/document.rs`, `src/export/tests.rs` |

Rules of the waves:

- At most three agents that run cargo work at the same time. The third starts only with at least 8 GB of free RAM. D2 and C1 run no cargo. Wave 4 has two cargo agents: W2 and D1. Wave 5 has two: H2 and W4. Wave 6 has two: X3 and P1. Their file sets are disjoint.
- A shared file has one owner in each wave. The owner is the agent whose set names the file. Another agent reports the entry that it needs.
- A wave starts after the green CI of the wave before it. After a wave with `[hard]` tasks, it also waits for a verifier report (T2, `opus` xhigh, read only, no cargo) without CRITICAL and HIGH.
- Wave 1 changes three crates. Wave 2 changes `recipe-domain` in one agent only, so X2 has no parallel writer in its crate.
- X1 keeps the value of `ParameterEntry` private. X2 adds the accessors after X1.
- E1 adds only the dev-dependency. E3 adds `serde` and `serde_json` to `[dependencies]` in wave 2.
- A1 owns every caller of the two use cases. W2 builds them with the adapter of `src/pages/export.rs`.
- An agent that sees a compile error in a file of another agent of its wave waits and runs again. After three runs it reports. The precedent is step P3.4.
- W4 runs in wave 5 after W2, because its layer covers the rows of W2. H2 and W4 have disjoint sets.
- W2 and W3 reuse `seed_approval_recipes` and `recipe_world`. They add a seed only when a state is missing.
- A T3 task that fails twice goes to T1-C with both reports. A red CI that one fix does not make green goes to a T1-O diagnosis (read only).
- Each code agent runs its one task through `/ccx:loop --min-severity low --loops 4` in single mode. Codex only reviews. After the approval of Codex, a Claude reviewer CR-<agent> (T2, `opus` xhigh, read only) reviews the files of the agent.

### D14. Test data

- `recipe-export` golden bodies (E2): `tests/golden.rs` builds the body of the example of `api.md` 6.2 and the body of a `Retired` version 2 of `R-100` with an `ARRAY[1..3] OF INT` parameter. It uses the public constructors of `recipe-domain`. These constructors take values of `uuid`, `time`, and `rust_decimal`, so the three crates go into `[dev-dependencies]` of `recipe-export` with `workspace = true` (user decision after the verifier of wave 2). E3b adds `uuid` in wave 2 for the tests of `error_body`. E2 adds `time` and `rust_decimal`. Test code may use `unwrap`. When a constructor is not public, E2 stops and reports.
- `recipe-app` memory fixtures (A1): the user rows of the approvers, so `version_detail` finds each login.
- `recipe-web` seeds (W2, W3): `seed_approval_recipes` and `recipe_world` first. A new seed is an `Approved` or a `Retired` version with the real stored hash, built through the commands. The memory seed with the zero hash (`memory/recipe/seed.rs`) gives the `E-EXP-002` case.
- The QA seed of step A6 extends `target/tools/seed_recipes.py` in the scratchpad.

### D15. Bound on the parallel exports

ADR-0056 gives the rules. The code places in `recipe-app`:

- `src/blocking.rs`: the gate of the exports, next to the diff gate. `Blocking::rebuild_export` moves the permit into the blocking task.
- `AppSettings` in `src/blocking.rs` holds `export.permits`. `AppContext::new` in `src/context.rs` builds one gate from it.
- `src/export/flow.rs`: `export_version` waits for the permit inside `timeout.export`, before the first read opens the transaction.
- `src/export/document.rs`: `write_document` passes the permit to `Blocking::rebuild_export`.

T15 of D11 gives the tests. P1 owns task 5.2 in wave 6.

## Risks / Trade-offs

- [The writer and the schema drift apart] → the golden tests of T5 check each golden document against the schema and compare the bytes.
- [The member order breaks the hash] → T1 keeps the stored hash vectors equal.
- [The `MIT-0` licence of `borrow-or-share`] → D4: the agent stops, and the user decides.
- [A long export with 262 144 leaves (A26)] → the writer runs in the rebuild task inside `timeout.export`. `export.permits` bounds the parallel export tasks (D15). The load test of step P4.2 measures it.
- [Six waves with shared files] → one owner per wave (D13).

## Migration Plan

No migration, no grant, no index. `reliability.md` section 7.11 gets one new setting, `export.permits` (ADR-0056).

## Open Questions

None. Each later doubt goes to the orchestrator, who records it here.

The user decided the findings of the step-6 reviews on 2026-10-23 (D1). No question stays open.
