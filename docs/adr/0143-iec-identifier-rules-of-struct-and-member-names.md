# ADR-0143: IEC identifier rules of struct and member names

Status: Accepted. Amends ADR-0024 (duplicate member names) and ADR-0120 (a) (earlier rule set)
Date: 2026-11-07
Deciders: project owner

## Context

The PLC data type review of 2026-11-06 found a fault in the names of structs and members. `docs/plan/review/2026-11-06-plc-type-review.md` holds it as #9.

These facts shape the decision:

- IEC 61131-3 compares identifiers without case. The Beckhoff page "Identifier" of TwinCAT 3 says that `VAR1` and `var1` are one name.
- `TypeName::parse` compares a struct name with the primitive names in exact case. Thus `Int`, `real`, and `Time_Of_Day` are valid struct names.
- `TypeName::parse` reserves no keyword. Thus `ARRAY`, `OF`, `STRUCT`, and `END_STRUCT` are valid struct names.
- `check_struct` of `PlcType` compares member names in exact case. Thus `speed` and `Speed` can be two members of one struct.
- `plc_type_named_unique` and `plc_type_member_name_unique` compare names in exact case. Thus `st_zone` and `ST_Zone` can be two structs of one machine.
- A PLC project refuses such a struct when the transfer system writes it. The editor accepts it and exports it.
- The stored data can hold names from before this decision. ADR-0120 gives the earlier rule set for a stored part that a command only reads.
- `plc_type` and `plc_type_member` are small tables. A plain index build on them takes a short `SHARE` lock.

The project owner decided on 2026-11-06 (fix plan, question Q-D, option (1)). On 2026-11-07 the owner also decided the keyword sources, the reference rule of item (i1), and the index build of item (k).

## Decision drivers

- An exported type is valid in the PLC project of the machine (R11, R12, R13).
- The domain checks all external input (R38).
- A stored struct stays readable, and a parameter that uses it keeps it (R19, ADR-0120).
- Each refusal uses a code of `reliability.md` §1.2 (R36).
- A failed upgrade changes nothing, and the retry of ADR-0031 can run it again (R31).

## Considered options

### Option A: Compare without case and reserve the keywords and the primitive names

- Pros: the rules of IEC 61131-3. The PLC project accepts every new name.
- Cons: a stored struct with a reserved name gets no new revision. Migration `0011` builds two indexes under a `SHARE` lock.

### Option B: Reserve only `ARRAY`, `OF`, `STRUCT`, and `END_STRUCT`

- Pros: a short list. No migration.
- Cons: `Int`, `real`, and the pair `speed` and `Speed` stay valid. The PLC project refuses them.

### Option C: No change

- Pros: no work.
- Cons: the editor keeps exporting types that the PLC project refuses.

## Decision

The project uses **Option A**.

- (a) A struct name compares without case on one machine. A new struct whose name equals the name of a stored struct of the machine in any case gives `E-PLT-015`. For example, `st_zone` next to `ST_Zone` gives `E-PLT-015`.
- (b) A member name compares without case in one struct. The members `speed` and `Speed` in one struct give `E-PLT-021`. This item amends ADR-0024.
- (c) A struct name is not a reserved word, in any case. The reserved words are the keywords of item (d) and the names of the primitives of `data-model.md` §7.
- (d) The keyword list is the union of two sources, without the primitive names. A word that is in only one source stays in the list. The project read both sources on 2026-11-07:
  - The Beckhoff Information System, TwinCAT 3 PLC, page "Keywords" (`https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529971595.html`). The page says that a keyword cannot be a variable name. It gives 118 words that are not primitive names.
  - The keyword list of IEC 61131-3 Ed. 3.0 (2013) §6.1.3, as the truST specification "Lexical Elements" reproduces it (`https://johannespettersson80.github.io/trust-platform/reference/specifications/01-lexical-elements/`). The project uses the groups from "Data Types" to "Special" and not the group of truST extensions. They give 124 words that are not primitive names.
  - Both sources hold 60 of the words. The list has these 182 words:

  ```text
  ABS
  ABSTRACT
  ACOS
  ACTION
  ADD
  ADR
  AND
  ANDN
  ANY
  ANY_BIT
  ANY_CHAR
  ANY_CHARS
  ANY_DATE
  ANY_DERIVED
  ANY_DURATION
  ANY_ELEMENTARY
  ANY_INT
  ANY_MAGNITUDE
  ANY_NUM
  ANY_REAL
  ANY_SIGNED
  ANY_STRING
  ANY_UNSIGNED
  ARRAY
  ASIN
  AT
  ATAN
  BITADR
  BY
  CAL
  CALC
  CALCN
  CASE
  CLASS
  CONFIGURATION
  CONSTANT
  CONTINUE
  COS
  DIV
  DO
  DT
  ELSE
  ELSIF
  EN
  END_ACTION
  END_CASE
  END_CLASS
  END_CONFIGURATION
  END_FOR
  END_FUNCTION
  END_FUNCTION_BLOCK
  END_GET
  END_IF
  END_INTERFACE
  END_METHOD
  END_NAMESPACE
  END_PROGRAM
  END_PROPERTY
  END_REPEAT
  END_RESOURCE
  END_SET
  END_STEP
  END_STRUCT
  END_TRANSITION
  END_TYPE
  END_VAR
  END_WHILE
  ENO
  EQ
  EXIT
  EXP
  EXPT
  EXTENDS
  FALSE
  FINAL
  FOR
  FROM
  FUNCTION
  FUNCTION_BLOCK
  F_EDGE
  GE
  GET
  GT
  IF
  IMPLEMENTS
  INDEXOF
  INITIAL_STEP
  INTERFACE
  INTERNAL
  JMP
  JMPC
  JMPCN
  LD
  LDATE_AND_TIME
  LDN
  LE
  LN
  LOG
  LT
  LTIME_OF_DAY
  MAX
  METHOD
  MIN
  MOD
  MOVE
  MUL
  MUX
  NAMESPACE
  NE
  NON_RETAIN
  NOT
  NULL
  OF
  ON
  OR
  ORN
  OVERLAP
  OVERRIDE
  PARAMS
  PERSISTENT
  POINTER
  PRIVATE
  PROGRAM
  PROPERTY
  PROTECTED
  PUBLIC
  R
  READ_ONLY
  READ_WRITE
  REF
  REFERENCE
  REF_TO
  REPEAT
  RESOURCE
  RET
  RETAIN
  RETC
  RETCN
  RETURN
  ROL
  ROR
  R_EDGE
  S
  SEL
  SET
  SHL
  SHR
  SIN
  SIZEOF
  SQRT
  ST
  STEP
  STN
  STRUCT
  SUB
  SUPER
  TAN
  TASK
  THEN
  THIS
  TO
  TOD
  TRANSITION
  TRUE
  TRUNC
  TYPE
  UNTIL
  USING
  VAR
  VAR_ACCESS
  VAR_CONFIG
  VAR_EXTERNAL
  VAR_GLOBAL
  VAR_INPUT
  VAR_IN_OUT
  VAR_OUTPUT
  VAR_STAT
  VAR_TEMP
  WHILE
  WITH
  XOR
  XORN
  ```
- (e) The 29 primitive names of `data-model.md` §7 complete the list. Thus the list has 211 words. `recipe-domain` holds the one copy of the list. The ops query of item (k) holds a second copy, and a test compares both copies.
- (f) Codes. The form of a new struct gives `E-WEB-008` with the field `type_name` for a reserved name. A new row of a struct with a reserved name gives `E-PLT-013` with the reason "reserved name". This case is a new revision of a stored struct. A type text of item (i2) that names such a struct gives the same code and reason. ADR-0025 makes the reason a text, not a code. Thus no code is new.
- (g) The lookup of a struct stays exact. The stored spelling is the only spelling of a struct. A `TypeExpr` `st_zone@1` finds no struct when the machine holds `ST_Zone`, and it gives `E-PLT-013`.
- (h) The class-2 key of a struct keeps the exact name (ADR-0144). Two commands that create `st_zone` and `ST_Zone` at one time take two keys. The index of item (j) refuses the second insert, and `recipe-app` maps it to `E-PLT-015`.
- (i) Earlier rule set. A stored struct name passes the earlier rule when it matches the grammar and is not a primitive name in exact case. Thus a stored struct `Int` stays readable. Member names compare without case in both sets, because the indexes of item (j) hold for every stored row. This item amends ADR-0120 (a).
- (i1) Three paths read a stored name or a stored type, so they use the earlier rule. The paths are the type rows of `recipe-db`, the audit labels, and the path value of a type page. A stored parameter type also uses the earlier rule, when a page shows it or a command keeps it.
- (i2) A command checks a type text that it writes with the current rule (ADR-0120 (c)). The texts are the type of `AddParameter`, a changed type of `UpdateParameterDefinition`, and the member types of `DefineUdt`. Thus a new parameter cannot name the stored struct `Int@1`. A parameter that names `Int@1` keeps it while its type does not change. The type proposals of the parameter form and the struct form leave out a stored struct with a reserved name.
- (i3) A command checks a new struct row with the current rule. Thus `DefineUdt` refuses a new revision of a stored struct with a reserved name.
- (j) Two new unique indexes enforce items (a) and (b) in the database:
  - `plc_type_struct_name_ci_unique` on `plc_type (machine_id, lower(name), revision) WHERE kind = 'struct'`
  - `plc_type_member_name_ci_unique` on `plc_type_member (struct_type_id, lower(member_name))`
- (j1) The struct index holds `revision`. Thus the database refuses two struct names that differ only in case within one revision of one machine. The domain refuses the pair across revisions, because a new name always starts at revision 1 and a new revision uses the exact stored name.
- (j2) The constraints `plc_type_named_unique` and `plc_type_member_name_unique` stay. The grammar CHECK allows only ASCII names, and the cluster uses `--lc-ctype=C.UTF-8`. Thus `lower` changes only the letters `A` to `Z`.
- (k) Migration `0011` runs in one transaction. It first takes a `SHARE` lock on `plc_type` and on `plc_type_member`. Then it stops when the stored data holds two struct names or two member names that differ only in case. Then it builds both indexes with `CREATE UNIQUE INDEX` and without `CONCURRENTLY`.
- (k1) The list `EXEMPT` of `deploy/ci/migration_rewrite.py` gets two entries, `("0011_plc_type_rules.sql", "index without concurrently")` and `("0011_plc_type_rules.sql", "lock table")`. Each entry has the reason `ADR-0143` and follows the rules of ADR-0136. Each entry keys on the file and the form. The first entry permits both indexes. The second entry permits the `LOCK TABLE` statement of item (k).
- (k2) A lock timeout (`55P03`) rolls back the whole file. It can also hit the `LOCK TABLE` statement while an old `app` transaction writes to `plc_type`. The retry loop of ADR-0031 then runs the file again. The ops query `deploy/db/check-identifier-rules.sql` lists the rows that stop the file and every stored struct with a reserved name.
- (l) A member name can be a keyword or a primitive name. The rule of item (c) is for struct names only.

## Consequences

### Positive

- Every new struct name and every new member list is valid in the PLC project of the machine.
- The database refuses two member names of one struct that differ only in case. It also refuses two struct names of one revision that differ only in case, also when two commands run at one time.
- The upgrade has no state between the check and the indexes, because the `SHARE` lock keeps writers out until the file ends. A failed run of `0011` leaves no index and no history row.

### Negative

- A stored struct with a reserved name gets no new revision. The user defines a struct with a new name. Then `UpdateParameterDefinition` moves each parameter to it.
- A stored parameter whose type names a struct with a reserved name keeps that type. The export of its version still holds the struct.
- The upgrade stops at migration `0011` when the stored data holds a case duplicate. A forward correction then needs a new decision.
- The index build blocks writes to `plc_type` and `plc_type_member` while it runs.
- The texts of `reason-type-name` change in the bundles `pl` and `en`. The bundles get the reason "reserved name".

### Risks and mitigations

- Risk: the IEC list comes from a public reproduction and not from the text of the standard, because that text is not free. Mitigation: the union with the Beckhoff list covers each word of TwinCAT 3. A new ADR can add words, and the list lives in one place of the code.
- Risk: the list does not hold keywords of Siemens SCL, for example `BEGIN` or `DATA_BLOCK`. Mitigation: vendor point R4 of the fix plan can add them with a new ADR.
- Risk: a long transaction of the old `app` holds a lock on `plc_type` past `migrate.lock_timeout`. Mitigation: the file rolls back, and the retry of ADR-0031 runs it again. After the last retry the upgrade stops, and the old `app` keeps the old schema.
- Risk: the entries `index without concurrently` and `lock table` of item (k1) also permit any other index without `CONCURRENTLY` and any other `LOCK TABLE` statement in `0011`. Mitigation: the review of the entries checks the whole file (ADR-0136, Risks).

## Related

- ADR-0024 (duplicate member names), ADR-0025 (reasons without a code), ADR-0031 (retry of the migrator), ADR-0120 (earlier rule set), ADR-0136 (rewrite forms and `EXEMPT`)
- ADR-0144 (order of the class-2 locks), ADR-0145 (lower subscript of the array bounds)
- `domain-model.md` §2.1 (`PlcType`, `TypeExpr`), `data-model.md` §3.3, §3.4, §8, `reliability.md` §4.3, §5
- Sources of item (d): the Beckhoff page "Keywords" and the truST page "Lexical Elements", read on 2026-11-07. Beckhoff page "Identifier", read on 2026-11-07
- `overview.md` §7: R11, R12, R13, R19, R31, R36, R38
