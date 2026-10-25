# ADR-0063: Webhook of the alerts

Status: Accepted. Amended by ADR-0075 (names of the alert conditions in the body). Amended by a note (items (f) and (n)). A mandatory filter of the log layer rejects every event and span of the HTTP client targets of `hyper_util` below `info`. The filter ignores `RUST_LOG`, also span and field directives. `build_filter` in `recipe-server` keeps fixed `info` directives for the same targets. No `RUST_LOG` value shows the host or the address of the webhook.
Date: 2026-10-23
Deciders: project owner

## Context

`reliability.md` section 3.7 lets the job `alerts` send one HTTP `POST` for each active condition to `alerts.webhook_url`. Step P4.1 builds the call, and the plan found these gaps:

- A webhook URL often holds a token. It is a credential, but section 7.11 makes it a plain configuration value. Section 3.4 does not list it.
- `app_setting` stores the effective value of every configuration key (`data-model.md` section 3.17, section 7.12, ADR-0022). The audit log keeps each `SettingChanged` forever, also in every dump.
- The plant can use its own certificate authority. `rustls` has no built-in roots, and no setting names a file of roots.
- The call has no timeout key. Ten conditions in sequence with 10 s each take 100 s, above `alerts.interval` (60 s).
- The network `backend` has `internal: true` (ADR-0070). From it, the job cannot reach a host outside the Compose networks.
- An unsalted SHA-256 of a URL with a short token is open to an offline guess.

## Decision drivers

- A secret never appears in a configuration file, an environment value, a log line, or the audit log (ADR-0031, section 3.4).
- A change of the webhook still has an audit entry (A34).
- A failed call does not stop the job (section 3.7, R37).
- No new error code.
- Every number comes from section 7.11.

## Considered options

### Option A: The URL in a secret file, a fingerprint in `app_setting`, HTTPS only

- Pros: the URL stays out of the configuration, the log, and the audit log. A change of the file still gives `SettingChanged`.
- Cons: one more secret file and one more key for the roots.

### Option B: The URL in the TOML file, with a fingerprint in `app_setting`

- Pros: no secret file.
- Cons: the token is in a configuration file, against ADR-0031.

### Option C: No webhook in step P4.1

- Pros: no code and no outbound network.
- Cons: section 3.7 changes, and the plant gets the alerts only from the event log.

## Decision

The project uses **Option A**.

Source and record:

- (a) The secret file of `alerts.webhook_url_file` holds the URL. The key is optional, and its default is `/run/secrets/alerts_webhook_url`. An empty key or an empty file at the path of the key means no webhook. The start then writes one `INFO` line that names the key. A missing file at a set path stops the start with one `ERROR` line, as a missing key file does.
- (b) The file holds one line, as ADR-0031 says. A URL that does not parse, a scheme other than `https`, a carriage return, or a second line stops the start. The start then writes one `ERROR` line that names the key only.
- (c) `app_setting` and `SettingChanged` hold the derived key `alerts.webhook_url`. Its value is `sha256:` and 64 lower-case hex characters of the URL. It is not a configuration key. With no webhook, the row holds the canonical text of an empty value of ADR-0062, the empty text.
- (d) For this one key, the stored value is the fingerprint and not the effective value. `data-model.md` section 3.17 and `reliability.md` section 7.12 get this sentence.
- (e) The runbook requires a token of at least 128 random bits in the URL. No code checks this rule.

Redaction:

- (f) The URL joins the list of section 3.4. No log line holds a part of the URL: no host, no path, and no query.
- (g) The `Display` and `Debug` text of `WebhookFault` and of the type that holds the URL hold no part of the URL.

Transport:

- (h) The client is `hyper_util::client::legacy::Client` with `hyper-rustls`. It uses no default features of `hyper-rustls`, and the features `http1`, `tls12`, `ring`, and `webpki-tokio`.
- (i) `rustls` uses the features `ring`, `std`, and `tls12`. `http-body-util` builds the body. No `aws-lc` crate enters the graph.
- (j) Without a CA file, the connector uses `with_provider_and_webpki_roots` with the `ring` provider, which returns a `Result`. With a CA file, it uses `with_tls_config` with a `ClientConfig` of `ClientConfig::builder_with_provider(ring)`. That `ClientConfig` has a `RootCertStore` that holds only the certificates of the file.
- (k) Both connectors use `https_only()`. The client follows no redirect.

Calls:

- (l) The body is `{"alert":"<condition>"}` with `Content-Type: application/json`. The job sends one `POST` for each active condition of a run (section 3.7).
- (m) The calls of one run go out at the same time. One `alerts.webhook_timeout` bounds all of them.
- (n) A connection fault, a status outside `2xx`, a timeout, or a TLS fault is a `WebhookFault`. The job writes one `WARN` line without the URL and continues.
- (o) `WebhookFault` is a reason type without `code()`, as ADR-0025 permits. Section 1.1 lists it like `ValueFaults` (ADR-0060 (r)).
- (p) `app` is the only member of the network `egress` (ADR-0065). The runbook gives a host firewall rule that limits this traffic to the webhook host.

Settings:

- (q) Section 7.11 gets the row `alerts.webhook_timeout` with the value 10 s for all `POST` requests of one run.
- (r) Section 7.11 gets the row `alerts.webhook_url_file` with the value `/run/secrets/alerts_webhook_url`. The key is optional.
- (s) Section 7.11 gets the row `alerts.webhook_ca_file` with the value empty, which means the webpki roots.
- (t) The row `alerts.webhook_url` changes. Its value is the fingerprint `sha256:<64 hex>` of the URL in the file, or empty. It is not a configuration key.

## Consequences

### Positive

- The token of the URL stays out of the configuration, the log, the audit log, and the dumps.
- A change of the URL still gives one `SettingChanged` entry.
- One run of `alerts` ends within `alerts.webhook_timeout` of the calls, below `alerts.interval`.

### Negative

- The operator manages one more secret file, and for a plant authority one more file of roots.
- An audit entry shows that the URL changed, but not the new host.

### Risks and mitigations

- Risk: an error text quotes the bad URL. Mitigation: the tests check that each refusal names the key only. They also check the `Display` and `Debug` text of `WebhookFault` and of the URL holder.
- Risk: a short token in the URL. Mitigation: the runbook rule of item (e).

## Related

- ADR-0022 (stored settings). This ADR amends it.
- ADR-0025, ADR-0031, ADR-0060, ADR-0062, ADR-0065, ADR-0070
- `docs/architecture/reliability.md` section 1.1 (`WebhookFault`), sections 3.4, 3.7, 7.11, and 7.12
- `docs/architecture/data-model.md` section 3.17 (the derived key and its fingerprint)
- Baseline requirements: operations "Alert conditions", "Redaction in the event log", and "Startup settings comparison"
- `overview.md` section 7: R34, R37. Section 9: A34
