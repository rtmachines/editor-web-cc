# ADR-0131: Token of the language switch with a session

Status: Proposed
Date: 2026-11-05
Deciders: project owner

## Context

`POST /lang` sets the cookie `recipe_lang` and writes no database row (`api.md` §8). It works without a session, with a session, and in read-only mode.

`api.md` §7.3 says that `POST /login` and `POST /lang` without a session use the pre-session token `csrf_pre`. It does not say which token `POST /lang` accepts with a session. The route table of `api.md` §2 gives "`csrf_pre` or CSRF token".

The CSRF layer accepts either token on `POST /lang`, also with a valid session. Only the archived design of the change `2026-10-14-identity-and-sessions` states this rule. The review of 2026-11-04 found the gap (finding D3-1).

## Decision drivers

- The normative documents state the rule that the code applies.
- A change of the language does not change data.
- The language switch of a page with a session sends the session token (`web-ui` spec).

## Considered options

### Option A: Require the session token when a session exists

- Pros: the rule is the same as for every other route with a session.
- Cons: the code changes, and the security gain is nil. The route changes only a cookie of the browser.

### Option B: Record the rule "either token"

- Pros: no code change. The documents then match the code and the route table.
- Cons: `POST /lang` stays the one route that accepts `csrf_pre` with a session, after `POST /login`.

## Decision

The project uses **Option B**.

- (a) `POST /lang` accepts the CSRF token of the session or a valid `csrf_pre`, with or without a session.
- (b) The Origin check of `api.md` §7.3 applies as before.
- (c) `api.md` §7.3 states item (a).

## Consequences

### Positive

- The normative documents hold the rule. The archived design is no longer its only source.

### Negative

- None. The behavior does not change.

### Risks and mitigations

- Risk: a later route copies the rule "either token" for a change of data. Mitigation: item (a) names only `POST /lang`.

## Related

- ADR-0035 (pre-session token)
- `api.md` §2, §7.3, §8
- Change `low-round-c`, capability `identity-access`
- `overview.md` §7: R38
