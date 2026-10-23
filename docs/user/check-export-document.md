# Check an export document before a PLC load

## Purpose and reader

This guide shows how to check an export document of the recipe editor before you load the recipe into a PLC.

The reader is the author of a transfer system or an engineer who loads a file into a PLC. The check finds a file in which someone edited a field with "In hash: yes" (`api.md` section 6.1). The hash does not cover the fields with "In hash: no", for example `loadable`, `version.status`, `version.approved_by`, `exported_at`, and the names (`api.md` section 6.1). Step 2 finds a document that the transfer system must not load.

## Sources

This guide links to the rules and does not repeat them.

- The schema file [`export-v1.schema.json`](../../crates/recipe-export/schema/export-v1.schema.json) is the source of truth. When the schema and `api.md` differ, the schema wins.
- [`api.md`](../architecture/api.md) section 2.7 gives the two export routes.
- [`api.md`](../architecture/api.md) section 5.3 gives the JSON error answer of an export route.
- [`api.md`](../architecture/api.md) section 6.1 gives the fields and the column "In hash".
- [`api.md`](../architecture/api.md) section 6.3 gives the array order.
- [`domain-model.md`](../architecture/domain-model.md) section 2.1 gives the base unit of each time value.
- [ADR-0019](../adr/0019-export-integrity-and-contract.md) gives the reason for the hash.

## Prerequisites

You need these tools:

- A validator for JSON Schema draft 2020-12.
- A library for the canonical form of RFC 8785.
- A function for SHA-256.
- A JSON parser that keeps strings and integers exact. Every number in the hashed body is an integer below 2^53 or a JSON string (`api.md` section 6.1).

## Procedure

1. Check the document against the schema file.
2. Check that `schema_version` is `"1"` and that `loadable` is `true`. Compare `version.number` with the expected version.
3. Build the hashed body from the document fields with "In hash: yes". Keep the array order and each `null`.
4. Write the canonical form of the hashed body. Compute its SHA-256 as lower-case hex.
5. Compare the hash with `content_sha256`. The two values must be equal.

Step 2 uses the field `loadable` of [`api.md`](../architecture/api.md) section 6.1. Step 3 uses the column "In hash" of section 6.1 and the array order of section 6.3. Section 6.3 requires the transfer system to check `loadable` and the hash before a PLC load.

## Test vector

Test your tool on the golden files of `recipe-export` before you use it on a real document:

- [`recipe_R-100_v3.json`](../../crates/recipe-export/tests/golden/recipe_R-100_v3.json) is an `Approved` version. Its `loadable` is `true`.
- [`RETIRED_recipe_R-100_v2.json`](../../crates/recipe-export/tests/golden/RETIRED_recipe_R-100_v2.json) is a `Retired` version. Its `loadable` is `false`.

Run steps 3 to 5 on each file. Your hash must equal the `content_sha256` of the file. If it does not, fix your tool first.

## What to report when a step fails

Do not load the recipe. Report these items:

- The number of the step that failed.
- The file name and the value of `version.number`.
- For step 1, the path and the text of the first schema error.
- For step 2, the value of `schema_version` or `loadable`, or the two version numbers.
- For steps 3 to 5, the computed hash and the value of `content_sha256`.

A hash difference may mean that someone edited the file after the download (ADR-0019). It may also mean that your tool changed the array order (`api.md` section 6.3).

An export route may return a JSON error and not a document. A request without a valid session gets `303` to `/login` and no JSON (`api.md` section 5.3). The JSON answer holds a `code`, a `message`, and a `request_id` (`api.md` section 5.3). Include these three values in the report.
