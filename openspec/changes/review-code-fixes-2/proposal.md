# Proposal

## Why

The review of 2026-11-21 (`docs/plan/review/2026-11-21-full-repo-review.md` §4) reports code faults that need no design decision. `docs/plan/review/2026-11-21-fix-plan.md` §3.1 gives 20 of them to this change. The design checks each fault at HEAD `20d21fb`. Two faults fail the gate and move to change B. The owner also moves V-28 there.

The change satisfies R10, R12, R28, R30, R31, R32, R34, R35, R36, R38, and R39. It uses A17, A22, A33, A35, A44, and A47.

## What Changes

- V-05: a save of the same type without value and limit fields keeps the stored value.
- V-09: `ListParameters` ends its transaction before it resolves the type trees.
- V-11, V-12: the S7 nesting check uses `checked_add`. `for_platform` reports a missing field of the mode before the mode checks.
- V-13: `recipe-db` and the memory adapter build each stored connection through `for_platform`.
- V-14: an IPv4-mapped trusted-proxy entry matches the canonical peer address.
- V-21: `SecurityEvent::forbidden` takes an `IpAddr`.
- V-26: `recipe-web` takes the distinct-tag count of `E-RCP-020` from `recipe-domain`.
- V-29: the answer of a failed `POST /login` has no language switch.
- V-32 to V-35: four tests end on an event and not on a fixed wait. The XSS guard checks the test attribute and each `NoText` reason.
- V-36: the reader of a result file opens it without a link and checks the open descriptor.
- V-10, V-19, V-25: doc comments and the ADR index follow the code and the ADR files.
- `CHANGELOG.md` gets one line under `[Unreleased]` for each changed behavior.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: "Language switch on full pages" replaces "Language switch". A failed `POST /login` gets no switch.
- `identity-access`: the requirement "Client address from trusted proxies" gets a scenario for an IPv4-mapped entry.

## Impact

- Code in `recipe-domain`, `recipe-app`, `recipe-db`, `recipe-web`, `recipe-server`, and `e2e/`.
- Documents: `docs/adr/README.md` and `CHANGELOG.md`.
- No change of a route, a migration, a setting, an error code, a dependency, or `docs/architecture/`.

## Out of scope

- V-18 moves to change B `review-design-fixes-2`. It needs a policy grammar and an ADR.
- V-20 moves to change B. It needs a new `E-PLT-020` variant or an ADR-0028 amendment.
- V-28 moves to change B. `CONTRIBUTING.md` line 77 has no fault, because `deny.toml` has `[bans]`. Change B adds the banned edge to `reliability.md` §8.
- V-05 with value fields and without `limits[i]`. Change B decides it.
- The findings of change B and change C `review-deploy-fixes-2`.
