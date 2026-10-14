# UX brief: recipe editor

Status: non-normative input of P3.1. This file lives in `docs/plan/` and binds no reviewer. `docs/architecture/design-system.md` holds the binding rules. The proto-personas come from `general-draft.txt` and `api.md` section 3.3. No interview supports them yet. A usability test with 3 to 5 plant users validates them before P3.3.

Outcome: a user never mistakes a `Draft` for an `Approved` version, and every result or error tells the user the next step (R36, R39).

## Proto-personas

| Persona | Role | Context of use | Goal | Main risk |
|---|---|---|---|---|
| `akowal` | Administrator | Office PC, 24-inch monitor, mouse and keyboard. Rare, high-impact tasks | Keep users, machines, and released versions correct | A destructive action without a clear confirmation |
| `anowak` | ProcessEngineer | Office PC and a laptop at the line. Long sessions on drafts with up to 4 000 parameters (A19) | Change values and limits fast and approve the drafts of others | A lost edit, a misread number, an unclear conflict |
| `mhoffman` | Operator | Touch panel at the machine, 60 to 80 cm away, standing, gloves, glare, noise, interruptions | Find the approved version, read its values, export it | A `Draft` read as `Approved`, a target too small for a glove |

## Five key tasks

| Task | Actor | Steps | Moment of truth | Design need |
|---|---|---|---|---|
| 1. Edit a parameter in a draft | `anowak` | Open recipe, open draft, find parameter, change value or limit, save | The save result: a toast with the new row version, or `conflict.html` that keeps the input | Table, form with field errors per leaf path, toast, conflict fragment, status badge in the page header |
| 2. Approve a version | `anowak` or `akowal`, not the draft editor | Open the approve preview, read the diff and the outdated parameters, approve | The page shows `Approved` and the number of the retired version | Diff table, two status badges, one primary button with `hx-sync`, error fragment for `E-RCP-015` |
| 3. Compare two versions | All roles | Pick versions `n` and `m`, read the diff | Each change is clear without color | Diff markers with icon and text: added, removed, changed. Old and new values side by side in mono |
| 4. Add a UDT | `anowak` or `akowal` | Open machine, open types, enter name and members, create | The type tree of the new revision | Repeating form rows, type expression hint, one error line per member, tree |
| 5. Export a version | All roles. `Retired` only with `VersionExportHistory` | Open recipe, open the approved version, export | The JSON file and its content hash | Status badge, export button, content hash in mono, error fragment for `E-EXP-001` |

## Design implications for P3.1

1. Status always shows a color, an icon, and a word. Color alone carries no meaning.
2. Codes, values, limits, references, logins, and addresses use a face that separates `0` and `O`, and `1`, `l`, and `I`.
3. A touch target is large enough for a gloved finger. A focus ring is visible on every background.
4. Every page has one error panel at the top of the content. It shows the code, one sentence, and the request id.
5. A result is visible in place: a toast for a success, a fragment for an error. No result disappears before the user can read it.
