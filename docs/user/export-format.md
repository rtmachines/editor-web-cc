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
- `loadable`: Tells whether the file marks the version as loadable.
- `content_sha256`: The hash of the hashed body.
- `machine`: The machine of the recipe.
- `recipe`: The recipe that the version belongs to.
- `version`: The exported version of the recipe.
- `types`: The types that the parameters use.
- `parameters`: The parameters with their type, value, and limits.

## Related pages

- [Check an export document before a PLC load](check-export-document.md) shows how to check a file.
- [How to export a version](how-to-export-a-version.md) shows how to export a version.
