# Design

## Context

`proposal.md` gives the motivation. `docs/plan/review/2026-11-21-full-repo-review.md` §4 gives the location, the failure scenario, and the verifier note of each finding. The design read each finding in the code at HEAD `20d21fb`. The code matches the review text, with the notes below.

Each row of `2026-11-21-fix-plan.md` §3.1 must pass four checks to stay in this change:

1. No new error code in `reliability.md` §1.2.
2. No new key in `reliability.md` §7.11.
3. No new dependency and no new feature of a dependency in a `Cargo.toml`.
4. No change of `docs/architecture/` and no new ADR.

| Finding | Result |
|---|---|
| V-05, V-09, V-10, V-11, V-12, V-13, V-14, V-19, V-21, V-26, V-29, V-32, V-33, V-34, V-35, V-36, V-25 | Pass. This change |
| V-18 | Fails check 4. Moves to change B by an owner decision. See "Findings left out" |
| V-20 | Fails check 4. Moves to change B by an owner decision. See "Findings left out" |
| V-28 | Moves to change B by an owner decision. See "Findings left out" |

## Goals / Non-Goals

**Goals:**

- Each fix keeps the rules of `reliability.md` §1 and §2 and `.claude/rules/web.md`.
- Each task starts with a test that fails on the defect, or states why no such test exists and names a mutation.

**Non-Goals:**

- No change of `docs/architecture/` or of an ADR file. No new ADR.
- V-18, V-20, and V-28. The owner moves them to change B `review-design-fixes-2`.

## Answers to the gate questions

- V-20: no existing variant fits. `PlantError::UnitNotFound { path }` (`E-PLT-020`) needs a `UnitPath`. The `LiveUnits` of another machine does not know the unit of a parameter, and a draft without parameters has no unit. `SnapshotItem::UnknownUnit` is the item that HEAD already gives inside `E-RCP-020`.
- V-11: an overflow returns the existing `PlantError::TypeTooLarge { type_name, limit: TypeLimit::S7Nesting }` (`E-PLT-018`), the same value as a count above the limit.
- V-18: no existing error fits. `IdentityError` has no variant for a bad policy. A reason type without a code needs a caller that wraps it into a coded error, and `begin_attempt` has no such error. No document states the valid range of the four values of `ThrottlePolicy`.
- V-36: the crate `rustix` gives `OFlags::NOFOLLOW`, `OFlags::NONBLOCK`, `fs::open`, and `fs::fstat`. `rustix` 1.1.6 is a direct Unix dependency of `recipe-server` with the features `fs` and `std` (ADR-0060 (c), (cc)). The four items are in the feature `fs`. `deny.toml` has no ban on `rustix`, and `cargo deny` already accepts it.
- V-14: no document states the stored form of a trusted-proxy entry. ADR-0062 gives the text in `app_setting`: "the CIDR texts joined by `,` in the given order". `config/serve.rs` writes that text from the raw texts, so D7 does not change it. `api.md` §7.1 says that the server maps every address to its canonical form.

## Decisions

### D1. No new value without a value field (V-05)

- `update_input` in `pages/parameters/edit.rs` passes `retype: None` when three facts hold. The sent `type` equals the stored type text of the parameter. The request has no value field. The request has no field `limits[i]`.
- "The stored type" is `form.type_text` of the form query, not the hidden field `value_type`. `prepare` already answers the form again when `value_type` differs from `type` (ADR-0039 (i)). A request without `value_type` therefore never reaches the command.
- `set_definition` keeps the value and the limits for `retype: None` (`api.md` §2.6, ADR-0029). The scenario "Same type without a value" of `recipe-management` states this behavior. No delta is necessary.

### D2. End the transaction before the resolve (V-09)

- `ListParameters::execute` reads the values with `read_row_values` and then drops the transaction. `PlatformTypes::collect`, `row_trees`, and `texts_of` run after `drop(tx)` (ADR-0042 (f)). `read_row_values` needs only the row keys.
- A source test in `queries/source_tests.rs` asserts the order for each `execute` body of `parameters.rs`: `drop(tx)` comes before `PlatformTypes::collect(`.

### D3. Doc comments (V-10, V-19)

- V-10: the comment of `TryFrom<StoredMachine>` names the code order. The order is the data block rule, the encoding rule, duplicate path, missing parent, archived parent of a live unit, and no live main unit.
- V-19: `check_definition` in `draft/s7.rs` gets `# Errors` with `E-RCP-003`, `E-RCP-053`, `E-RCP-057`, `E-RCP-054`, `E-RCP-023`, `E-RCP-056`, and `E-RCP-058` in the order of its rules. The first group of `add_parameter` and of `check_new_definition` adds `E-RCP-057`, `E-RCP-056`, and `E-RCP-058`. `check_for_approval` adds `E-RCP-058`.

### D4. Checked nesting count (V-11)

- `check_s7_nesting_of_expr` returns `Ok(())` at once on a platform that is not an S7 platform. On an S7 platform it uses `checked_add`. `None` gives the error of the gate answer. A non-S7 platform therefore keeps "accepts any count".
- HEAD and the fix give the same result for each input, because a saturated count is above `MAX_S7_NESTING`. The task names a mutation in place of a red test.

### D5. Check order of the connection (V-12)

- `Faults::check_credentials` splits into two steps. The step of the missing fields records each mode-dependent `E-PLT-023`. It runs after the required fields and before `check_mode` (step 2 of `domain-model.md` §2.1). The step of the refused fields records `E-PLT-028` and stays last (step 4).
- The existing test `a_request_with_faults_of_every_kind_lists_all_of_them_in_check_order` gets the new order. No spec scenario states the order.

### D6. Stored connections through `for_platform` (V-13)

- `build_connection` of `recipe-db` builds `ConnectionFields` from the row and calls `PlcConnection::for_platform` with the machine code and the stored password state. A `PlantErrors` maps to the existing `corrupt(TABLE, ..)` error. The caller drops the password plan.
- The machine code comes from a scalar subquery `(SELECT m.code FROM machine m WHERE m.id = $1)` in the three statements. A scalar subquery takes no row lock, so `FOR UPDATE` and `FOR SHARE` still lock only the `plc_connection` row. A join would also lock the `machine` row.
- `ConnectionRow::stored` of the memory adapter reads the code from its `MachineRow` and returns `Result<StoredConnection, StorageError>`. `ShapeRow::to_connection` goes away.
- The `CHECK` constraints of `migrations/0012_plc_connection.sql` refuse each row that `for_platform` refuses. A test therefore calls the private builder with a hand-built record. The test does not go through the database.

### D7. Canonical trusted-proxy entry (V-14)

- `Cidr::parse` maps an IPv4-mapped IPv6 network with a prefix of 96 or more to the IPv4 network with the prefix minus 96. `::ffff:172.28.0.2/128` becomes `172.28.0.2/32`.
- A mapped network with a prefix below 96 stays an IPv6 network. It also covers addresses that are not mapped, and no IPv4 form exists. Such an entry matches no canonical IPv4 peer, as at HEAD.
- The mapping uses `Ipv6Addr::to_ipv4_mapped`, like `IpAddr::to_canonical` in `client.rs`. `::1/128` therefore stays IPv6, because `to_ipv4_mapped` gives `None` for it.
- The doc comment of `contains` names the canonical entry. The new scenario of the `identity-access` delta states the result.

### D8. Forbidden event with an address (V-21)

- `SecurityEvent::forbidden` takes `client_ip: IpAddr`, like the other request constructors.
- `refuse` in `auth/authorize.rs` returns `StorageError::invariant` (`E-DB-008`) and writes no event when `RequestMeta` has no address. The requirement "Forbidden result event" of `identity-access` gives the code of the storage error when the event cannot be written. The web layer always passes an address, because the layer `client_address` answers `E-WEB-010` without one.
- The 11 test callers of `forbidden` pass a plain `IpAddr`.

### D9. One distinct-tag count (V-26)

- `recipe-domain` gets `pub fn distinct_tag_count(items: &[SnapshotItem]) -> usize` in the module `recipe::snapshot`. The `Display` of `E-RCP-020` and the Fluent argument `count` of `recipe-web` call it. The copy in `error/recipe.rs` goes away.

### D10. No switch on a failed login (V-29)

- `login_page_with_error` builds the layout with `next: None`. The page keeps `pre_nonce`, so the login form still sends `csrf_pre`. `Layout::for_login` stays for `GET /login`.
- This follows `design-system.md` §10.3 and ADR-0036 (j). The `web-ui` delta changes the requirement sentence and renames the scenario of a failed login to "No switch after a failed login".
- The validator refuses a renamed scenario inside a MODIFIED block. The delta therefore removes the requirement "Language switch" and adds "Language switch on full pages" with the same three other scenarios.

### D11. Tests that end on an event (V-32, V-33, V-35)

- Rule: a test waits on events. It may bound one wait with `recv_timeout` of `EVENT_WAIT`. It never calls `thread::sleep`.
- V-32: the watchdog is a `std` thread on `recv_timeout`, so paused `tokio` time does not apply. The tests use the gated fake of `start_gated` and need no change of production code.
  - Disarm: arm, receive `started`, call `disarm()`, and open the gate. Then `started.recv_timeout(k * interval)` gives `Timeout`, with `k` a named constant above 1. Then `stop_bounded` joins the thread.
  - Arm again: arm, receive `started`, disarm, and open the gate. Then arm again and receive a second `started` within `EVENT_WAIT`.
  - Drop: arm, receive `started`, drop the handle, and open the gate. The thread ends, and `started.recv_timeout(EVENT_WAIT)` gives `Disconnected` and no second start.
  - Not armed: the test waits for one full interval of the thread before it sends `Stop` through `stop_bounded`, and asserts that `started` holds no start. An unarmed loop has no event that a test can observe.
- V-33: a helper thread runs `wait_for_signal` and sends the server back on a channel. The test waits with `recv_timeout(EVENT_WAIT)`, so a lost signal fails with a message. It asserts `stop_cause() == Some(StopCause::Signal)` before `close`.
- V-35: the e2e test waits for `location.hash` to equal `#parameter-save-submit`. Then the page sends one sentinel `fetch` of `/static/app.css` with `cache: 'no-store'`, so the browser cache cannot hide it. The test waits for that request to finish and asserts that the request list holds only the sentinel. `NO_REQUEST_WINDOW_MS` goes away.

### D12. XSS guard (V-34)

- Corpus: `has_test_fn` accepts `fn name(` only when the attribute group above it holds `#[test]` or `#[tokio::test]` and no `#[cfg(` other than `#[cfg(test)]`. A `#[cfg]` on the enclosing module stays out of scope. A corpus fn does not count when its attribute group holds `#[ignore]` or `#[cfg_attr(...)]` naming `ignore`.
- `NoText`: every `NoText` field that holds text is a `String` (for example `role` of `users.rs` and `page` of `users/query.rs`). A check by field type alone therefore refuses all these entries or permits all of them.
- New rule, owner approves with `tasks.md`: `Cover::NoText` holds a closed kind. The scan records the Rust name and the type text of each field. Each kind has one check.
  - `Unread`: the Rust name starts with `_`, so no handler can read the field. Each CSRF field uses this kind.
  - `Typed`: the type is an integer type or `bool`, also inside `Option`. The `row_version` fields of `approve.rs` and `actions.rs` and `allow_outdated` use this kind.
  - `Secret`: the type is `SecretField` or `Option<SecretField>`. The PLC password uses this kind.
  - `Parsed(path)`: the path names the function that turns the `String` into a value that is not text. The module of the struct holds a read `.<field>` and the path on the same line or on the next three lines. That window holds no read of another field of the struct. A call site compiles only when the function exists.
- Each current entry passes. `Parsed` gets these paths: `path_values::page`, `Lang::parse`, `Role::parse` (two entries), `reference`, `parse_encoding`, `StringEncoding::parse`, `Rack::parse`, `Slot::parse`, `TcpPort::parse`, `CommMode::parse`, and `parse::<i64>` for `ConnectionRequest.row_version`.
- A wrong entry fails. A `String` field with `Typed` gives a fault. A `Parsed` path that the module does not call next to a read of that field gives a fault.

### D13. Result file without a link (V-36)

- On Unix, `read_result_file` opens the path with `RDONLY | NOFOLLOW | NONBLOCK | CLOEXEC` through `rustix::fs::open`. `NONBLOCK` keeps a FIFO from blocking the open. `ELOOP` gives `Reading::Rejected`. `ENOENT` gives `Reading::Missing`.
- `fstat` of the open descriptor decides. A file that is not a regular file, or a file above `MAX_RESULT_BYTES`, gives `Reading::Rejected`. The read uses `File::from(OwnedFd)`. No `unsafe` code is necessary.
- On other systems the current path with `symlink_metadata` stays, like the `DiskSpace` adapter (ADR-0060 (c)).
- A rename race has no deterministic test. The new opener has unit tests on a link and on a FIFO.

### D14. ADR index (V-25)

The five rows of `docs/adr/README.md` copy the "Amended by ADR-0174" note of the Status line of their ADR file.

## Findings left out

The owner decided on 2026-11-21 that these three findings move to change B `review-design-fixes-2`. No task of this change touches them.

- V-18: `parse` needs a rule for a bad `ThrottlePolicy`. `domain-model.md` §2 is the authority for each grammar (`reliability.md` §4.3), and no document gives a range of the four values. A bad policy also needs a coded error for `begin_attempt` or a startup check. The fix changes `domain-model.md` §2.3 and needs an ADR. Owner decision: V-18 moves to change B.
- V-20: a refusal with `E-PLT-020` needs a new variant or a new field, because `UnitNotFound` names a path. That changes the owner cell of `E-PLT-020` in `reliability.md` §1.2. The other path is an amendment of ADR-0028. Owner decision: V-20 moves to change B. The tests `units_of_another_machine_give_e_plt_020_items` in `snapshot/tests.rs` and `draft/approval/tests.rs` pin the HEAD behavior.
- V-28: `CONTRIBUTING.md` line 77 has no fault, because `deny.toml` has `[bans]`. Owner decision: V-28 moves to change B. The amendments ADR of change B adds "a banned dependency edge" to `reliability.md` §8.

The code review of 2026-11-21 left four open points for change B `review-design-fixes-2`:

- A-05: The `Parsed` check matches by module and field name, so two structs with one field name share one check.
- A-09: The code can read a field that starts with `_`, so the `Unread` kind can be wrong.
- A-10: A `Corpus` entry can name the test of another field. This gap is older than this change.
- A-11: The sentinel request of the e2e test misses a request that starts after the sentinel.

## Settled points

The plan review of 2026-11-21 found these three decisions right.

- V-12: a reader can put the mode-dependent login of `domain-model.md` §2.1 at step 2 or at step 4. D5 puts it at step 2.
- V-14: a mapped entry with a prefix below 96 stays IPv6 and matches no canonical peer (D7).
- V-21: a refusal without a client address gives `E-DB-008` and writes no event (D8).

## Open points for change B

- V-05: D1 stays as it is. A save of the same type with value fields and without `limits[i]` is open. The amendments ADR of change B states whether the value and the limits of `api.md` §2.6 are optional only as a pair.

## Risks / Trade-offs

- [D6] A stored row that `for_platform` refuses now gives `E-DB-008` on the read. The `CHECK` constraints prevent such a row, so no live data changes.
- [D11 V-32] The unarmed test waits one full interval of the thread before it sends `Stop`. That wait adds run time to the test.
- [D12] The scan reads the type text of a field. A type alias hides the real type. → `Typed` and `Secret` accept only the named type texts.
- [D12] The window of `Parsed` reads source text. A call that spans more than four lines fails the check. → The author moves the path next to the read.
- [Tasks] Several tasks edit one file. → `tasks.md` gives the order of these tasks.
