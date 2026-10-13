# Design: baseline-specs

## Context

`proposal.md`, section Why, gives the motivation. The documents in `docs/architecture/` and `docs/adr/` are normative. The specs of this change copy their rules and cite the R-numbers, A-numbers, and I-numbers of these documents.

## Goals / Non-Goals

**Goals:**

- Each capability names the document sections that hold its rules.

**Non-Goals:**

- New design decisions. This change adds no ADR and changes no file in `docs/`.

## Decisions

This change makes no design decision. The table names the sources of each capability. `docs/adr/README.md`, section Decision map, names the current ADR of each topic.

| Capability | Sections in `docs/architecture/` | Current ADRs | Scenario sources |
|---|---|---|---|
| `plant-model` | `domain-model.md` 2.1, 3, 5.1. `data-model.md` 3.1 to 3.4, 4.2, 4.5, 4.6, 4.8, 6, 7. `reliability.md` 1.2, 4.3, 7.11 | ADR-0006, ADR-0017, ADR-0018, ADR-0021 | Invariants of `domain-model.md` 3. Rows of `data-model.md` 6 |
| `recipe-management` | `domain-model.md` 2.1, 2.2, 3, 4, 5.2. `data-model.md` 3.9 to 3.14, 4.1, 4.3, 4.4, 4.7, 4.8, 4.10, 6. `reliability.md` 1.2, 4.3, 4.4, 7.11 | ADR-0007, ADR-0008, ADR-0009, ADR-0010, ADR-0012, ADR-0015, ADR-0016, ADR-0018, ADR-0021, ADR-0022 | Invariants of `domain-model.md` 3. Rows of `data-model.md` 6 |
| `identity-access` | `domain-model.md` 2.3, 5.3, 5.5. `api.md` 3, 7. `data-model.md` 3.5 to 3.7, 6. `reliability.md` 1.2, 7.11 | ADR-0013, ADR-0014, ADR-0022 | Rows of `data-model.md` 6. `api.md` 3 and 7 |
| `audit` | `domain-model.md` 2.4, 3, 5. `data-model.md` 3.8, 3.15, 3.16, 4.9, 4.12, 5.2, 6, 10. `reliability.md` 1.2, 3.6, 7.11 | ADR-0005, ADR-0022 | Invariant I13. Rows of `data-model.md` 6. `data-model.md` 3.8, 4.9, 4.12, 10 |
| `export` | `api.md` 2.7, 6. `domain-model.md` 5.4. `data-model.md` 6. `reliability.md` 1.2 | ADR-0019 | Row `ExportVersion` of `data-model.md` 6. `api.md` 6 |
| `web-ui` | `api.md` 4, 5, 8. `reliability.md` 1.2, 7.11, 8 | ADR-0001, ADR-0008, ADR-0012, ADR-0015, ADR-0023 | `api.md` 4, 5, 8. `reliability.md` 8. Invariant I12 |
| `operations` | `overview.md` 3. `reliability.md` 1.1, 2.4, 3, 5, 7. `data-model.md` 6 | ADR-0020, ADR-0022 | `reliability.md` 1.1, 2.4, 3, 5, 7. Row `cleanup` of `data-model.md` 6 |

The specs do not use the superseded ADR-0002, ADR-0003, ADR-0004, and ADR-0011.

## Risks / Trade-offs

- A spec sentence can differ from the document that it copies. → The documents stay normative. P1.2 checks each requirement against its source.
- Two capabilities can state the same rule. → P1.2 runs a redundancy check across the seven specs.
