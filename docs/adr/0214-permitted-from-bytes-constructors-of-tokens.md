# ADR-0214: Permitted `from_bytes` constructors of `SessionToken` and `CsrfToken`

Status: Accepted. Amends ADR-0166 (u)
Date: 2026-11-22
Deciders: project owner

## Context

The full repository review of 2026-11-21 found a second public constructor on two token types. `docs/plan/review/2026-11-21-full-repo-review.md` §4 holds the finding.

These facts shape the decision:

- `reliability.md` §4.3 says that each value object has one constructor `parse`. ADR-0166 permits `from_stored` and names other constructors in its items (u) to (x).
- ADR-0166 (u) names `OneTimePassword::from_bytes` only. That constructor builds the password from random bytes that the application supplies.
- `SessionToken` and `CsrfToken` in `crates/recipe-domain/src/identity/secret.rs` also have a public `from_bytes`. No document names these two constructors.
- A reviewer can thus refuse valid code. A new type can also copy an unlisted constructor, because two unlisted constructors already exist.
- A caller can build a token from a constant array and skip the random source. Outside tests, no caller does this today.

## Decision drivers

- A session token and a CSRF token come from a random source (R21, `api.md` §7.1, §7.3).
- A value object checks its external input in its constructor (R38).
- A rule of the documents matches the shipped code. Thus a review does not refuse valid code.
- `recipe-domain` does no I/O. It cannot read the random source itself.

## Considered options

### Option A: Name the two `from_bytes` constructors in the pattern of ADR-0166 (u)

- Pros: the code keeps its behavior. The documents match the code.
- Cons: the list of constructors other than `parse` gets two more items.

### Option B: Move the random source into `recipe-domain`

- Pros: no constructor takes bytes from a caller.
- Cons: `recipe-domain` then does I/O. The tests of other crates lose a fixed token.

### Option C: Build each new token with `parse` from hex text

- Pros: one constructor for each token type.
- Cons: the caller encodes random bytes as text only to decode them again. A constant text still skips the random source.

## Decision

The project uses **Option A**.

- (a) `SessionToken::from_bytes` is a permitted second constructor. It builds the token from 16 random bytes that the application supplies. This is the 128-bit token of `api.md` §7.1.
- (b) `CsrfToken::from_bytes` is a permitted second constructor. It builds the token from 32 random bytes that the application supplies. The `session` row holds the token as 64 hex characters (`data-model.md` §3.6).
- (c) Each constructor accepts every array of its length. It checks nothing, because every array of that length is a valid token.
- (d) Outside tests, only `IssuedSession::generate` in `crates/recipe-app/src/identity/issue.rs` calls the two constructors. It reads the bytes from the random source of the operating system.
- (e) A failed read of the random source gives `AppError::TaskFailed` with the stage `Token` (`E-WEB-011`, `reliability.md` §1.2).
- (f) Neither constructor reads a stored value or a cookie value. A stored value and a cookie value go through `parse`.
- (g) `crates/recipe-db/src/session_store.rs` reads the stored CSRF token with `CsrfToken::parse`. `crates/recipe-web/src/session.rs` and `crates/recipe-web/src/pages/request.rs` read the cookie with `SessionToken::parse`.
- (h) Code outside tests passes no constant array and no derived array to the two constructors. Test code can build a fixed token from a constant array.
- (i) This ADR amends ADR-0166 (u). The list of other constructors now names three `from_bytes` constructors: `OneTimePassword`, `SessionToken`, and `CsrfToken`.
- (j) The code keeps its behavior. The doc comments of the two constructors cite this ADR.

## Consequences

### Positive

- A review accepts the two constructors of the token types.
- A reader finds the random source of each token in one ADR.

### Negative

- The rule of one constructor for each value object gets two more exceptions.

### Risks and mitigations

- Risk: a new caller outside tests passes a constant array and skips the random source. Mitigation: item (h) forbids it, and a review refuses such a caller.
- Risk: a new caller passes stored bytes to `from_bytes` and skips the format check. Mitigation: item (f) sends a stored value and a cookie value through `parse`.

## Related

- ADR-0166 (u) (the pattern of `OneTimePassword::from_bytes`), ADR-0035 (the session and the CSRF token)
- `reliability.md` §1.2, §4.3
- `api.md` §7.1, §7.3, `data-model.md` §3.6
- Change `review-design-fixes-2`
- `overview.md` §7: R21, R38
