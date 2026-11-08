# WP7 R1: S7 layout of the composite rows against the Siemens documentation

Date: 2026-11-08. Scope: row R1 of section 10 of `docs/plan/2026-11-06-plc-type-fix-plan.md`, finding 16, Q09, and Q10.

This record is research only. It changes no code. It is not normative.

## Method

1. The table `crates/recipe-domain/src/plant/s7_layout/reference.rs` has 24 composite rows: C01 to C13 and A01 to A11. The count agrees with the fix plan.
2. The research read the Siemens TIA documentation portal (`docs.tia.siemens.cloud`, TIA Portal V21) through the CLI client. It used 24 calls of the client.
3. For each row, the research derived the size, the start class, and the member offsets from the cited rules. Then it compared them with the literals of the row.
4. A verdict uses only a Siemens rule. A rule that comes from an example has `medium` confidence at most.

No TIA Portal measurement exists. A `consistent` verdict means "consistent with the documentation", not `Verified`.

## Rules

Each quote is verbatim. "Data types (V21)" is the publication "Data types", TIA Portal V21, locale en-US, unless the row says de-DE.

| Id | Rule | Quote | Source | Confidence |
|---|---|---|---|---|
| S-ST1 | A struct starts at an even byte and ends at an even byte. | "A STRUCT tag always starts in a non-optimized block at a word limit, i.e. at a byte with even address. The individual components are then located in the order of their declaration in the memory. STRUCT tags occupy the memory up to the next word limit." | Data types (V21), "STRUCT data structure (anonymous structures) > Structure of a STRUCT tag", `tia:bjsfqzHlsPj3vP_iLZCazQ/pvr16j0OfvxqukWkREXpRw` | high |
| S-ST2 | Start of a member in a struct, by data type. | "Components with BOOL data type start in the least significant bit. Components with BYTE and CHAR data type start in the next byte. Components with different data type start at a word limit." | Same page as S-ST1 | high for the text. See S-ST2a and S-ST2b for the reading |
| S-ST2-de | The German text of S-ST2. It says "of the next byte" for `BOOL`. | "Komponenten mit Datentyp BOOL beginnen im niederwertigsten Bit des nächsten Bytes. Komponenten mit Datentyp BYTE und CHAR beginnen im nächsten BYTE. Komponenten mit anderen Datentypen beginnen an einer Wortgrenze." | Datentypen (V21), de-DE, "Datenstruktur STRUCT (Anonyme Strukturen) > Aufbau einer STRUCT-Variablen", `tia:HYGLwrX~QBzHY9zNMnrHfg/ZCPyqOsJWDDz2ZM9jardZg` | high for the text |
| S-ST2a | Consecutive `BOOL` members of a struct share one byte. S-ST2 does not state it. An example of the S7-1200 manual implies it. | "Another example is a DWord tag overlaid with a Struct. The Struct includes a Word, Byte, and two Booleans" and "The size of the overlaying parameter must be less than or equal to the size of the overlaid parameter." | S7-1200 Programmable controller (system manual, the portal shows no version), "PLC concepts > Data types > Accessing a tag with an AT overlay", `tia:urp2Ni2UpoHXhbA9nhgm6A/m3wXTQ0xij_wofPm0hx7hw` | medium. Derivation: WORD 2 bytes, BYTE 1 byte, so the two `BOOL` fit into 4 bytes only when they share byte 3 |
| S-ST2b | `SINT` and `USINT` are not in the list "BYTE and CHAR" of S-ST2. Read literally, they "start at a word limit". | "An operand of data type SINT occupies one BYTE in the memory." No page in reach gives the start of a `SINT` or a `USINT` member. | Data types (V21), "Integers > SINT (8-bit integers)", `tia:bjsfqzHlsPj3vP_iLZCazQ/7HRJ3kpPfwv2oTeSQkr9Pg`, with S-ST2 | medium |
| S-PAD | Every structured data type (struct, PLC data type, array) starts at an even byte and ends at an even byte. A padding byte fills the gap. | "Structured data types are aligned in the non-optimized memory area to an even start address (divisible by 2)." and "If, for example, an array with a length of seven BYTES follows three single BYTES in a data block, a padding byte is inserted automatically both before the array and after the array. The first three BYTES are located, for example, at the addresses "0.0" to "2.0". The subsequent array is then located at the addresses "4.0" to "10.0". A single BYTE that follows would get the address "12.0"." The table row of the page names "Struct PLC data types Array". | Programming basics (V21), "Serializing structures for data exchange > Padding bytes when using structured data types", `tia:ZxnBzLCX~KQ8x~1dsG1Sig/K8ecwR0vEpFWM4doKkSlFA` | high |
| S-AR1 | An array starts at an even byte and ends at an even byte. The English text says "never". The German text says "immer" (always). | en-US: "With non-optimized blocks, an ARRAY tag never starts at a word limit, i.e. at a byte with even address. ARRAY tags occupy the memory up to the next word limit." de-DE: "Eine ARRAY-Variable beginnt bei nicht optimierten Bausteinen immer an einer Wortgrenze, d. h. an einem Byte mit gerader Adresse. ARRAY-Variablen belegen den Speicher bis zur nächsten Wortgrenze." | Data types (V21), "ARRAY > Structure of an ARRAY tag", `tia:bjsfqzHlsPj3vP_iLZCazQ/DACOpzCWpCpqs2zhR2O~gA`. Datentypen (V21), de-DE, "ARRAY > Aufbau einer ARRAY-Variablen", `tia:HYGLwrX~QBzHY9zNMnrHfg/Hs41lx0FAF4SniRYBd2TJQ` | high. The German text and S-PAD agree. The English "never" is a fault of the translation |
| S-AR2 | Elements of a one-dimensional array follow each other. `BOOL` elements start at bit 0. `BYTE` and `CHAR` elements take one byte each. | "Components with BOOL data type start in the least significant bit. Components with BYTE and CHAR data type start in the right byte. The individual components are listed in order." | Same page as S-AR1 (en-US) | high |
| S-AR3 | In standard access, a `BOOL` element of an array takes 1 bit. The text states only the optimized case. | "For a block with the "Optimized block access" block property, an element of the BOOL data type requires 1 byte of memory. This is also true when you use an ARRAY of <data type>. An ARRAY[0..1] of BOOL, for example, thus requires 2 bytes in an optimized program block." and "An array of Booleans is an overlay for the byte tag B1" | Data types (V21), "ARRAY > Basic information on ARRAY", `tia:bjsfqzHlsPj3vP_iLZCazQ/NrYQ9PwPVSlqrRVktIJHBQ`. S7-1200 manual, AT overlay page of S-ST2a | medium |
| S-AR4 | In a multi-dimensional array, each new line starts at the next byte for bit and byte elements, and at the next even byte for other elements. | en-US: "In multidimensional arrays, the components are stored line-by-line (dimension-by-dimension), starting with the first dimension. In bit and byte components a new dimension always starts at the next byte. In components of other data types a new dimension always starts in the next word (in the next even byte) in non-optimized blocks." de-DE: "Eine neue Dimension beginnt bei Bit- und Byte-Komponenten immer im nächsten Byte." | Same pages as S-AR1 | high for the text. medium for the reading of "a new dimension" in 3 dimensions (see A03) |
| S-AR5 | The element of an array cannot be an array in a block with standard access. | "All data types except ARRAY are permitted for the components." The table gives for "Standard": "All data types except ARRAY Multi-instances". | Data types (V21), "ARRAY > Basic information on ARRAY", `tia:bjsfqzHlsPj3vP_iLZCazQ/NrYQ9PwPVSlqrRVktIJHBQ` | high |
| S-STR1 | `STRING[n]` takes n + 2 bytes. Byte 0 holds the maximum length. Byte 1 holds the current length. | "An operand of the STRING data type occupies two bytes more than the specified maximum length in the memory." and "The maximum length is located in the first byte of the character string. The current length is in the second byte." | Data types (V21), "Character strings > STRING", `tia:bjsfqzHlsPj3vP_iLZCazQ/Ff_XAGZLkjHsdKXn8BugVw`, and "Structure of a STRING tag", `tia:bjsfqzHlsPj3vP_iLZCazQ/mCV2w6t2YzeiwLBFuDoNcw` | high |
| S-STR2 | A `STRING` starts at an even byte. | "It starts in a non-optimized block at a word limit (at a byte with even address). In an optimized block it can start at any byte limit." | "Structure of a STRING tag", as S-STR1 | high |
| S-WSTR | `WSTRING[n]` takes n + 2 words, that is 2n + 4 bytes. The page in reach gives no order of the two header words and no start rule. S-ST2 "different data type" gives the even start in a struct. | "An operand of the WSTRING data type occupies two WORDs more in the memory than the specified maximum length." | Data types (V21), "Character strings > WSTRING (S7-1200, S7-1500, S7-1200 G2)", `tia:bjsfqzHlsPj3vP_iLZCazQ/2aGl8UyORTpcjSu9SSw92g` | high for the size. No rule in reach for the header order |
| S-DTL | `DTL` takes 12 bytes. It is a predefined structure and a complex data type, so S-ST2 and S-PAD give an even start. | "An operand of data type DTL has a length of 12 bytes and stores date and time information in a predefined structure." The overview lists "Complex data types (DT, LDT, DTL, STRING, WSTRING, ARRAY, STRUCT)". | Data types (V21), "Date and time > DTL", `tia:bjsfqzHlsPj3vP_iLZCazQ/35FRkTORp08lthk7TgkSQw`, and "Overview of the valid data types", `tia:bjsfqzHlsPj3vP_iLZCazQ/noOksz6JEfWYU__wI3HMfA` | high |
| S-BOOL | A `BOOL` takes 1 bit in standard access. | "Length (bits) 1" in the property table, and "For a block with the block property "Optimized block access", the bit has a length of 1 byte." | Data types (V21), "Binary numbers > BOOL (bit)", `tia:bjsfqzHlsPj3vP_iLZCazQ/ZXq~A1QIAjMFu1AjAsLxWQ` | high |

No composite row uses `DT`, `LTIME`, `LDT`, or `LTOD`. The research did not check their sizes. S1 of `reference.rs` covers the primitive sizes.

Secondary source, not used for a verdict: the blog `industrialmonitordirect.com`, "Siemens STEP 7 Data Block Structure Offset Addressing Explained". Its language versions contradict each other on `BOOL` packing.

## Rows

The column "Expected" gives the value that the cited rules give. A member is `path byte.bit`.

| Row | Verdict | Cited rules | Note |
|---|---|---|---|
| C01 struct of 9 BOOL | consistent | S-ST1, S-ST2, S-ST2a, S-BOOL | `.a` to `.h` at 0.0 to 0.7, `.i` at 1.0, size 2. The packing of consecutive `BOOL` members rests on S-ST2a (medium) |
| C02 BOOL, BYTE | consistent | S-ST1, S-ST2 | `.a` 0.0, `.b` "in the next byte" 1.0, size 2 |
| C03 BOOL, INT | consistent | S-ST1, S-ST2 | `.b` "at a word limit" 2.0, size 4 |
| C04 BYTE, CHAR, SINT, USINT | differs | S-ST1, S-ST2, S-ST2b | Expected `.d` 4.0 and size 6. Actual `.d` 3.0 and size 4. Derivation below. Confidence medium |
| C05 BYTE, STRING(3) | consistent | S-ST2, S-STR1, S-STR2, S-ST1 | `.b` 2.0, 2 + 5 = 7, rounded up to 8 |
| C06 BYTE, WSTRING(3) | consistent | S-ST2, S-WSTR, S-ST1 | `.b` 2.0 by "different data type", 2 + 10 = 12 |
| C07 WSTRING(1), BYTE | consistent | S-WSTR, S-ST2, S-ST1 | `WSTRING(1)` takes 6 bytes. `.b` at 6.0, 7 rounded up to 8 |
| C08 BYTE, ST_Flags | consistent | S-ST1, S-ST2, S-ST2a, S-PAD | `.b` 2.0, `.b.a` to `.b.c` 2.0 to 2.2. `ST_Flags` takes 2 bytes, so the size is 4 |
| C09 BOOL, ARRAY[1..2] OF BOOL | consistent | S-AR1, S-PAD, S-AR3 | `.b` 2.0. The array takes 2 bits, rounded up to 2 bytes. Size 4 |
| C10 BYTE, BYTE, BYTE | consistent | S-ST2, S-ST1 | 0.0, 1.0, 2.0. 3 bytes, rounded up to 4 |
| C11 STRING(3), BYTE | consistent | S-STR1, S-ST2, S-ST1 | `.b` 5.0, 6 bytes |
| C12 STRING(3), INT | consistent | S-STR1, S-ST2, S-ST1 | `.b` 6.0 at the next word limit, 8 bytes |
| C13 BYTE, DTL | consistent | S-DTL, S-ST2, S-ST1 | `.b` 2.0, 2 + 12 = 14 |
| A01 ARRAY[1..17] OF BOOL | consistent | S-AR1, S-AR2, S-AR3 | 17 bits fill 3 bytes, rounded up to 4 |
| A02 ARRAY[1..3,1..3] OF BOOL | differs | S-AR4, S-AR1 | Expected 4. Actual 2. Derivation below |
| A03 ARRAY[1..3,1..3,1..3] OF BOOL | differs | S-AR4, S-AR1 | Expected 10, or 6 under a second reading. Actual 4. Derivation below. Confidence medium |
| A04 ARRAY[1..3] OF BYTE | consistent | S-AR1, S-AR2, S-PAD | 3 bytes, rounded up to 4. S-PAD shows the same rule for 7 bytes |
| A05 ARRAY[1..3] OF CHAR | consistent | S-AR1, S-AR2 | 3 bytes, rounded up to 4 |
| A06 ARRAY[1..4] OF STRING(3) | consistent | S-STR1, S-STR2, S-AR1 | Each element takes 5 bytes and starts at a word limit. Elements at 0, 6, 12, 18. 23 bytes, rounded up to 24 |
| A07 ARRAY[1..2] OF WSTRING(3) | consistent | S-WSTR, S-AR1 | Each element takes 10 bytes. 20 bytes. The even start of an element follows from the even size |
| A08 ARRAY[0..1,1..2] OF INT | consistent | S-AR4, S-AR1 | Each line takes 4 bytes and starts at an even byte. 8 bytes |
| A09 ARRAY[1..2] OF ARRAY[1..3] OF BYTE | differs | S-AR5 | Expected: TIA Portal refuses the type in a block with standard access. Actual: a layout of 8 bytes. This answers Q09 |
| A10 ARRAY[1..3] OF ARRAY[1..3] OF BOOL | differs | S-AR5 | Expected: TIA Portal refuses the type. Actual: a layout of 6 bytes. This answers Q09 |
| A11 ARRAY[1..2] OF ST_Flags | consistent | S-ST1, S-PAD, S-ST2a | Each struct element starts at an even byte and takes 2 bytes. 4 bytes |

Counts: 19 `consistent`, 5 `differs`, 0 `no rule`.

### Derivation of C04

1. The struct starts at byte 0 (S-ST1).
2. `.a BYTE` starts at 0.0.
3. `.b CHAR` "start[s] in the next byte" (S-ST2). It starts at 1.0.
4. `.c SINT` is not `BOOL`, `BYTE`, or `CHAR`. It is a "different data type" and starts "at a word limit" (S-ST2b). The next word limit is byte 2. It starts at 2.0. The row agrees.
5. `.d USINT` is a "different data type" too. The next word limit after byte 2 is byte 4. It starts at 4.0. The row has 3.0.
6. The last byte is byte 4. "STRUCT tags occupy the memory up to the next word limit" (S-ST1). The size is 6. The row has 4.

The literal text names only `BYTE` and `CHAR` as byte-aligned. The text does not name `SINT` or `USINT` at all. A TIA measurement must decide this row. `domain-model.md` §2.1 and ADR-0017 permit a `SINT` and a `USINT` on any byte.

### Derivation of A02

1. The array starts at byte 0 (S-AR1).
2. The array has 3 lines of 3 `BOOL` elements, `[1,1..3]`, `[2,1..3]`, and `[3,1..3]` (S-AR4, "line-by-line").
3. Line 1 takes bits 0.0 to 0.2.
4. "In bit and byte components a new dimension always starts at the next byte" (S-AR4). Line 2 takes 1.0 to 1.2. Line 3 takes 2.0 to 2.2.
5. The last byte is byte 2. "ARRAY tags occupy the memory up to the next word limit" (S-AR1). The size is 4.
6. The row has 2. It packs the 9 bits without a line break.

### Derivation of A03

1. The array starts at byte 0 (S-AR1). It has 27 `BOOL` elements in 9 lines of 3 elements.
2. Reading 1: each line of the last index starts at the next byte (S-AR4). 9 lines take bytes 0 to 8. The next word limit gives a size of 10.
3. Reading 2: only a new value of the first index starts at the next byte. Each block of 9 bits takes 2 bytes. 3 blocks take bytes 0 to 5. The size is 6.
4. The row has 4. It packs the 27 bits without a line break. Both readings give another value.
5. The text does not decide between reading 1 and reading 2. A TIA measurement must decide the expected size.

### Derivation of A09 and A10

1. "All data types except ARRAY are permitted for the components" (S-AR5).
2. The element of A09 is `ARRAY[1..3] OF BYTE`. The element of A10 is `ARRAY[1..3] OF BOOL`.
3. TIA Portal therefore refuses both types in a block with standard access. No Siemens layout exists for them.
4. The rows pin a layout of 8 bytes and 6 bytes. The domain accepts both types.

## Impact

### What the change `s7-layout-reference-verified` must change

1. Add a source mark for "consistent with the documentation", for example `Source::Documented(..)`. Cite the rule ids and the page of this record. Do not use `Verified`. The owner accepted this status in place of a measurement.
2. Mark the 19 `consistent` rows with the new mark. The test `reference_table_marks_every_composite_row_unverified` must change with the marks.
3. Change the array rule of `S7Layout` for a multi-dimensional `BOOL` array. A new line starts at the next byte (S-AR4). A02 becomes 4 bytes. This needs a new ADR that amends ADR-0017 and the rule of `domain-model.md` §2.1 "In an array, `BOOL` elements pack 8 per byte".
4. Decide the 3-dimensional reading of S-AR4 for A03 (10 or 6 bytes). Without a measurement, A03 stays `Unverified`.
5. Decide `SINT` and `USINT` in a struct (C04). The literal text gives a word limit. `domain-model.md` §2.1 and ADR-0017 give any byte. The documents disagree, so the owner decides. A change needs a new ADR.
6. Decide the array of an array (A09, A10, Q09). S-AR5 refuses it in standard access. `domain-model.md` §2.1 permits "also another array" as an element. The coverage review links ADR-0045. Two options exist:
   - Refuse an array element of an array on the S7 platforms with an existing error code, and remove A09 and A10.
   - Keep the type and record in a new ADR that its S7 layout has no Siemens source.
7. The fix plan expects "A02 and A10 get one size". The documentation does not give one size. A02 becomes 4 bytes, and A10 has no Siemens layout.

### What stays open for a TIA measurement

- C04: the start of `SINT` and `USINT` members in a struct.
- A03: the size of a 3-dimensional `BOOL` array.
- C01, C08, A11: consecutive `BOOL` members in one byte. The source is an example (S-ST2a, medium).
- The order of the two header words of `WSTRING[n]`. No row depends on it.
- The start classes of the primitive rows P01 to P30 also have the mark `Unverified("Q10")`. S-ST2 supports `Even` for every type wider than 1 byte and `Byte` for `BYTE` and `CHAR`. P06 `SINT` and P07 `USINT` have the open point of C04.
