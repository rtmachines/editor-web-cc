# Transfer client guide

This guide gives the steps to set up and run a transfer client.
A transfer client is a program that pulls an `Approved` recipe from the recipe editor and loads it into a PLC.
The rules behind the steps are in ADR-0109, `api.md` sections 2.7, 6, and 7.7, and `reliability.md` section 7.11.
Replace each value in angle brackets or in capital letters.
The examples use the placeholders `recipe.example`, `M12`, `R-100`, and `U1.2`.

**Contents**

- [Before you start](#before-you-start)
- [Create the account](#create-the-account)
- [Rotate the password](#rotate-the-password)
- [Trust the CA and use port 443](#trust-the-ca-and-use-port-443)
- [Send the header on every request](#send-the-header-on-every-request)
- [The three routes](#the-three-routes)
- [Poll for a change](#poll-for-a-change)
- [Check a document before a load](#check-a-document-before-a-load)
- [Handle the response codes](#handle-the-response-codes)
- [Related](#related)

## Before you start

The client needs these items:

- The host name or address of the editor. The examples use `recipe.example`.
- The certificate of the plant CA that signed the certificate of Caddy.
- The login and the password of an account with the role `ExportClient`.
- A route from the client host to TCP port 443 of the editor.

The client uses HTTP Basic on the three routes of this guide.
The client never uses the session cookie and never opens the UI.
The account of the client holds only the permission `VersionExport`.
It cannot log in to the UI, and it cannot change data.

## Create the account

An Administrator creates one account for each client.
A person and a client never share one account.
The role of an account cannot change to `ExportClient` or from `ExportClient`.
The system gives `409 E-IDN-011` for such a change.

1. Log in to the UI as an Administrator.
2. Open `/users/new`.
3. Enter the login of the client, for example `plc-m12`.
4. Select the role `ExportClient`.
5. Submit the form.
6. Copy the generated password from the page.
7. Store the password in the secret store of the client host.

The page shows the password once.
The administrator does not choose the password.
The system does not set `must_change_password` for this account, so the generated password is the permanent credential of the client.
The login has 1 to 64 characters.
The client sends a password of 1 to 128 characters.

## Rotate the password

Rotate the password after each suspected leak.
Rotate it also after the client sent the password over plain HTTP.
Caddy refuses an `/export/` request over plain HTTP, but the client has already sent the password in that request.

1. Log in to the UI as an Administrator.
2. Open `/users/<login>` for the account of the client.
3. Use the reset password action of that page.
4. Copy the new password from the fragment.
5. Store the new password in the secret store of the client host.
6. Restart the client, or reload its credentials.
7. Check that the client reads the list route with the new password.

The old password stops working at once.
A request that runs during the reset may fail with `401 E-IDN-003`.
The reset does not set `must_change_password` for an `ExportClient` account.
To stop a client for good, an Administrator deactivates the account.

## Trust the CA and use port 443

Caddy handles TLS with a certificate of the plant PKI.
A public CA did not sign it.
The client must trust the plant CA, or every request fails the TLS handshake.

1. Ask the operator for the certificate of the plant CA.
2. Copy the certificate to the client host in PEM form.
3. Give the path of the file to the HTTP library of the client.
4. Use `https://` and the default port 443 in every address.
5. Ask the operator to permit TCP port 443 from the client host.

The HTTP site of Caddy answers each path under `/export/` with `403` and no redirect.
A client with an `http://` address therefore fails on the first request.
Do not disable the certificate check to work around a TLS error.
When a plant device stands in front of Caddy, the operator follows the section "A plant device in front of Caddy" of the [runbook](runbook.md#a-plant-device-in-front-of-caddy).

## Send the header on every request

The client sends `Authorization: Basic` on the first request.
It does not wait for a challenge.
A `401` answer has the header `WWW-Authenticate: RecipeExport realm="export"`.
A browser does not know this scheme.
An HTTP library that waits for a `Basic` challenge before it sends credentials therefore never connects.
Configure such a library for preemptive authentication.

The header value is `Basic ` and the Base64 text of `<login>:<password>`.
The Base64 text is standard Base64 with padding.
The system splits the decoded text at the first `:`, so the password can hold `:`.
The system refuses these cases with `401 E-WEB-012` and runs no password check:

- More than one `Authorization` header.
- A header longer than 1 024 bytes (`export.basic_header_max`).
- A scheme other than `Basic`.
- Text that is not standard Base64 with padding, or bytes that are not UTF-8.
- Decoded text without `:`.
- A login outside 1 to 64 characters, a password outside 1 to 128 characters, or a control character in either part.

Each request costs one password check on the server.
At most 1 Basic check waits for or holds a hash permit at the same time (`export.basic_permits`).
Send the requests of one client one after the other.

The following command shows the header with `curl`.
The option `--user` sends `Authorization: Basic` on the first request.
Store the password in the variable `EXPORT_PASSWORD` and never write it in a script.

```bash
curl --fail-with-body --cacert plant-ca.pem \
  --user "plc-m12:${EXPORT_PASSWORD}" \
  "https://recipe.example/export/v1/machines/M12/recipes"
```

The client sends no `HX-Request` header and no `Sec-Fetch-Site` header other than `same-origin` or `none`.
A navigation guard refuses such a request with `403 E-WEB-007` before the password check.
A program that is not a browser sends neither header.

## The three routes

All three routes use `GET` and return `application/json`.
The scheme of all three routes is HTTP Basic.
A request with a session cookie but without `Authorization` gets `401 E-WEB-012`.

| Route | Result |
|---|---|
| `GET /export/v1/machines/{code}/recipes` | The list of the recipes of the machine that have an `Approved` version. Writes no audit entry |
| `GET /export/v1/machines/{code}/recipes/{ref}/approved` | The v1 document of the `Approved` version of one recipe |
| `GET /export/v2/machines/{code}/recipes/{ref}/approved/units/{path}` | The v2 document of one unit and its descendant units |

The list body has the object `machine` and the array `recipes[]`.
The object `machine` holds `code`, `plc_platform`, and `archived`.
Each row of `recipes[]` holds `reference_number`, `name`, `version_number`, `approved_at`, and `content_sha256`.
The rows follow the byte order of the reference number.

The following commands show one request for each route.

```bash
BASE="https://recipe.example"
AUTH="plc-m12:${EXPORT_PASSWORD}"

curl --fail-with-body --cacert plant-ca.pem --user "$AUTH" \
  "$BASE/export/v1/machines/M12/recipes"

curl --fail-with-body --cacert plant-ca.pem --user "$AUTH" \
  "$BASE/export/v1/machines/M12/recipes/R-100/approved"

curl --fail-with-body --cacert plant-ca.pem --user "$AUTH" \
  "$BASE/export/v2/machines/M12/recipes/R-100/approved/units/U1.2"
```

The machine code names the live machine with that code.
When no live machine has the code, it names the archived machine that the system archived last.
A unit path is `U` and 1 to 5 numbers with a dot between two numbers.
A recipe of another machine gives `404 E-WEB-005`.

## Poll for a change

Each export writes one audit entry.
A client that exports on a timer fills the audit log.
Use the list route to find a change, and export only after a change.

1. Read the list route.
2. Find the row of your recipe by `reference_number`.
3. Compare `content_sha256` of the row with the value that you stored last.
4. If the values are equal, stop. Do not run an export.
5. If the values differ, run the export route.
6. Load the document only after the checks of the next section pass.
7. Store `content_sha256` of the document, not the value of the list row.

An approval can occur between the read of the list and the export.
The client therefore trusts `version.number` and `content_sha256` of the document, not the values of the list row.
Use the list row only to decide whether to run an export.
The system does not state a polling interval.
The operator chooses it. The list route writes no audit entry.

## Check a document before a load

The client must check each document before it loads the document into a PLC.
The checks differ a little for v1 and v2.

1. Check that `loadable` is `true`.
2. Build the hashed body from the document with the array order of `api.md` section 6.3.
3. Compute the SHA-256 of the hashed body.
4. For a v1 document, compare the result with `content_sha256`.
5. For a v2 document, build the scope hashed body and compare its hash with `scope_sha256`.
6. Load the document only when the hashes are equal.
7. Refuse the load and report the fault when a hash differs.

The v2 client cannot rebuild `content_sha256` from a v2 document.
A v2 document holds only the parameters of one unit.
The v2 client uses `content_sha256` only to find a change in any part of the version.
The scope hash is the lower-case hex SHA-256 of the RFC 8785 (JCS) canonical form of the scope hashed body.
`api.md` section 6.4 gives the fields of that body.

The client trusts `content_sha256` and `version.number` of the document.
It does not trust the list row for these values.

## Handle the response codes

The client handles these codes.
Every error body is JSON with the code `E-<CTX>-<nnn>`.

| Status | Code | Meaning and client action |
|---|---|---|
| `200` | - | The document or the list. Run the checks before a load |
| `401` | `E-WEB-012` | The header is not valid. Fix the header. Do not retry in a loop |
| `401` | `E-IDN-003` | Unknown login, inactive account, wrong password, or an account of a person. Ask the Administrator. Do not retry in a loop |
| `403` | `E-WEB-007` | The navigation guard refused the request. Remove the browser headers |
| `403` | none | Caddy refused a plain HTTP request. Use `https://` and rotate the password |
| `404` | `E-WEB-005` | No such machine, recipe, or unit. Check the values of the path |
| `404` | `E-EXP-001` | The recipe has no `Approved` version. Do not load |
| `404` | `E-EXP-005` | The unit has no parameter in its scope. Do not load |
| `422` | `E-PLT-017`, `E-PLT-002` | The unit path is not valid. Fix the path |
| `429` | `E-IDN-005` | Too many failed logins. Wait for the seconds in `Retry-After`, then retry |
| `500` | `E-EXP-002` | The content hash does not match. Do not load. Tell the Administrator |
| `500` | `E-EXP-004` | The server cannot create the document. Tell the Administrator |
| `503` | `E-WEB-009` | The request took too long or no hash permit was free. Wait for `Retry-After`, then retry |

A wrong password counts as a failed login.
The login delay and the budget per address of the UI login also apply to the Basic check.
A client with a wrong password therefore gets slower answers. After 100 failed attempts in 15 minutes, the address gets `429 E-IDN-005` for every login.
In read-only mode, the three routes stay available, so the client gets no `503 E-WEB-006`.

Every `401` answer has the header `WWW-Authenticate: RecipeExport realm="export"`.
The answer language follows the request.
The default language of the `message` field is `pl`.
Do not parse the message. Use the code.

## Related

- [ADR-0109](../adr/0109-export-api-for-transfer-clients.md): Export API for transfer clients with HTTP Basic
- [api.md](../architecture/api.md): sections 2.7, 6, and 7.7
- [reliability.md](../architecture/reliability.md): section 1.2 for the error codes, section 7.11 for the settings
- [Runbook](runbook.md): certificate rotation and a plant device in front of Caddy
