# Export document format

## Purpose and reader

This page is the entry point for the format of an export file. It shows the source document of each part of the format.

The reader is the author of a transfer system or an engineer who receives an export file.

## Where to find what

| Topic | Source |
|---|---|
| The two export routes (`/export/v1/`) | [`api.md`](../architecture/api.md) section 2.7 |
| File name and the `RETIRED_` prefix | [`api.md`](../architecture/api.md) section 6.3, [ADR-0019](../adr/0019-export-integrity-and-contract.md) |
| Fields and the column "In hash" | [`api.md`](../architecture/api.md) section 6.1 |
| Example document | [`api.md`](../architecture/api.md) section 6.2 |
| Order of the arrays | [`api.md`](../architecture/api.md) section 6.3 |
| JSON error answer of an export route | [`api.md`](../architecture/api.md) section 5.3, [ADR-0053](../adr/0053-error-answers-of-the-export-routes.md) |
| Schema file, the source of truth | [`export-v1.schema.json`](../../crates/recipe-export/schema/export-v1.schema.json) |
| Content hash and the reason for it | [ADR-0019](../adr/0019-export-integrity-and-contract.md), [`api.md`](../architecture/api.md) section 6.3 |
| Base unit of time values | [`domain-model.md`](../architecture/domain-model.md) section 2.1 |

## Top-level keys

The document has these top-level keys. The column "In hash" of [`api.md`](../architecture/api.md) section 6.1 names the fields that the hash covers.

- `schema_version`: The version of the document format.
- `exported_at`: The time of the export.
- `exported_by`: The login of the user who exported the version.
- `loadable`: `true` only when the version is `Approved` at export time and no value, limit, or address breaks a rule.
- `content_sha256`: The hash of the hashed body.
- `machine`: The machine of the recipe.
- `recipe`: The recipe that the version belongs to.
- `version`: The exported version of the recipe.
- `types`: The types that the parameters use.
- `parameters`: The parameters with their type, value, and limits.

## The content hash

The key `content_sha256` holds the hash of the hashed body. The value has 64 lower-case hexadecimal characters, from `0` to `9` and `a` to `f`.

The hashed body holds the fields that the column "In hash" of [`api.md`](../architecture/api.md) section 6.1 marks with "yes". Do not edit these fields. A changed value does not match the hash.

The hash does not cover the fields that the column marks with "no". These fields are `schema_version`, `exported_at`, `exported_by`, `loadable`, `content_sha256`, `machine.name`, `recipe.name`, `version.status`, `version.approved_at`, `version.approved_by`, and `parameters[].name_en`. A change in one of these fields does not change the hash.

The hash stays the same for one version. It does not change when you rename the recipe or the machine, or when you retire the version. The editor computes the hash at approval.

## Related pages

- [Check an export document before a PLC load](how-to-check-an-export-document.md) shows how to check a file.
- [How to export a version](how-to-export-a-version.md) shows how to export a version.
