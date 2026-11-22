# ADR-0215: Grammar of `ThrottlePolicy`

Status: Accepted. Amends `domain-model.md` §2.3 and `reliability.md` §1.2
Date: 2026-11-22
Deciders: project owner

## Context

The full repository review of 2026-11-21 found a public constructor that skips a check. `docs/plan/review/2026-11-21-full-repo-review.md` §4 holds the finding.

These facts shape the decision:

- `domain-model.md` §2.3 calls `LoginThrottle` a value object. `reliability.md` §4.3 gives each value object one constructor `parse` and no public `new`.
- `LoginThrottle` in `crates/recipe-domain/src/identity/throttle.rs` has a public `new(policy: ThrottlePolicy)` and no `parse`.
- `ThrottlePolicy` has four public fields: `free_failures`, `delay_cap`, `window`, and `address_budget`. No code checks these values.
- A value outside the rules of `reliability.md` §7.11 can thus reach the decision of the throttle and the attempt store.
- Today the only caller outside tests uses `ThrottlePolicy::default()` in `crates/recipe-app/src/identity/settings.rs`. So no wrong value reaches the throttle today. The rule conflict stays.

## Decision drivers

- A value object checks its external input in its constructor (R38, `reliability.md` §4.3).
- The decision and the attempt store read the same four values. One check must cover both readers.
- A rule of the documents matches the shipped code. Thus a review does not refuse valid code.
- Login throttling has a progressive delay and a budget for each address (A22, `api.md` §7.4).

## Considered options

### Option A: Give `ThrottlePolicy` a grammar and one `parse`, and build `LoginThrottle` from the parsed policy

- Pros: one check covers the decision and the attempt store. `LoginThrottle` gets no second check.
- Cons: the fields become private. Each reader of a field changes to an accessor. Some test literals change.

### Option B: Keep the code and change the text of `domain-model.md` §2.3

- Pros: no code changes.
- Cons: a policy outside the rules still reaches the decision. A later setting of the policy then has no check.

### Option C: Give `LoginThrottle` a `parse` with the four values

- Pros: the type that `domain-model.md` §2.3 names gets the constructor.
- Cons: the attempt store reads `ThrottlePolicy` and not `LoginThrottle`. A policy with no check still reaches the store.

## Decision

The project uses **Option A**.

- (a) `ThrottlePolicy` is the value object of the login throttle. It has one constructor `ThrottlePolicy::parse(free_failures, delay_cap, window, address_budget)`.
- (b) The grammar has three rules. Each rule includes both bounds:
  1. `free_failures` is from 1 to `address_budget` − 1.
  2. `delay_cap` is from 1 s to `window`.
  3. `window` is from 1 s to 24 h.
- (c) The defaults of `reliability.md` §7.11 pass the grammar. They are 3 free failures, a cap of 60 s, a window of 900 s, and a budget of 100.
- (d) The four fields of `ThrottlePolicy` become private. Each field gets an accessor without the prefix `get_`. Every reader outside the module uses the accessors.
- (e) A value that breaks the grammar gives the new code `E-IDN-012`, `IdentityError::ThrottlePolicyInvalid { field }`. The value `field` names the field that breaks the grammar.
- (f) `E-IDN-012` has the status `-` in `reliability.md` §1.2. No route reaches it, because the settings use the constant default.
- (g) A later setting of the policy reaches `E-IDN-012` only at startup. The row of `E-IDN-012` goes into `reliability.md` §1.2 before the variant goes into the code.
- (h) `LoginThrottle::new` goes. `LoginThrottle` implements `From<ThrottlePolicy>`, because the policy is already a parsed value. So `LoginThrottle` has no public constructor that skips the grammar.
- (i) `ThrottlePolicy::default()` stays a constant inside the module. It reads no external input and needs no `parse`. A unit test runs `parse` on the default values, so the default and the grammar cannot drift apart.
- (j) Each test literal of `ThrottlePolicy` outside the module changes to `parse`. A fixture with a small `address_budget` sets `free_failures` inside the grammar. For a budget of 2, `free_failures` is 1. For a budget of 3, it is at most 2.
- (k) `domain-model.md` §2.3 gets the grammar and names `ThrottlePolicy` as the value object with the one `parse`.

## Consequences

### Positive

- Every policy that reaches the decision or the attempt store passed the grammar.
- A later setting of the policy gets a check without a new constructor.
- A review accepts the shape of the two types.

### Negative

- The API of `recipe-domain` changes. Each reader of a field and each test literal of the policy changes in the same change.
- The exhaustive match of `crates/recipe-server/tests/error_registry/identity.rs` and the `IdentityError` arms of `crates/recipe-web/src/error/lists.rs` get the new variant.

### Risks and mitigations

- Risk: a later change edits the default and breaks the grammar. Mitigation: the unit test of item (i) fails.
- Risk: a later change adds a public constructor that skips `parse`. Mitigation: item (h) forbids it, and a review refuses such a constructor.

## Related

- ADR-0014 (the login throttle), ADR-0035 (the throttle of `ChangeOwnPassword`), ADR-0166 (other constructors of a value object)
- `domain-model.md` §2.3, `reliability.md` §1.2, §4.3, §7.11
- `api.md` §7.4
- Change `review-design-fixes-2`
- `overview.md` §7: R38. `overview.md` §9: A22
