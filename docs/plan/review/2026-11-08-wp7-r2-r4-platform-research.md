# WP7 vendor research: R2, R3, R4

Date: 2026-11-08. Scope: points R2, R3, and R4 of section 10 of `docs/plan/2026-11-06-plc-type-fix-plan.md`.
An agent did this research. It changed no code and no normative document.

## Sources and method

- The agent searched the Siemens TIA documentation portal (`docs.tia.siemens.cloud`) through the CLI client. A compact reference `tia:<map>/<topic>` names each page.
- The agent downloaded Siemens PDF documents from `cache.industry.siemens.com` and extracted their text.
- The agent could not read Siemens entry 67655611. The SIOS page needs JavaScript, and the backend API answered `403`. The agent did not try to work around the block.
- Quotes from Siemens stay verbatim. A quote in a table cell keeps the words of the cell.

| Id | Document | Version or date | Reference |
|---|---|---|---|
| S1 | Declaring the block interface, "Valid data types in the block interface (S7-1200, S7-1500)" | TIA Portal V21 | `tia:QndbLRb_mH6nT2SYzFQekA/wwX~ovzjkqEk8fbM7T2xwg` |
| S2 | Data types, "WSTRING (S7-1200, S7-1500)" | TIA Portal V20 | `tia:7MUllEJRyi4MtgzJh2qLpA/u_EpgsSn0GmbFVVjQ0uqMA` |
| S3 | Programming basics, "Basics of block access" | TIA Portal V21 | `tia:ZxnBzLCX~KQ8x~1dsG1Sig/Hv3Af5Fh8RhZZFkQq5iong` |
| S4 | Data types, "Overview of the valid data types" | TIA Portal V20 | `tia:7MUllEJRyi4MtgzJh2qLpA/jZht2ELpXnIXqd54FzXQUA` |
| S5 | Programming Guideline for S7-1200/1500, entry 81318674 | V1.6, 12/2018 | `https://cache.industry.siemens.com/dl/files/040/90885040/att_970576/v1/81318674_Programming_guideline_DOC_v16_en.pdf` |
| S6 | S7-1200 Programmable controller, "Character and String data types" | portal publication V20, "V4.7 release" | `tia:urp2Ni2UpoHXhbA9nhgm6A/XB0_rEcBSMoBNOa3hS5ZHw` |
| S7 | S7-1200 Easy Book, section 4.4.1 | 01/2015, A5E02486774-AG, firmware V4.1 | `https://cache.industry.siemens.com/dl/files/145/39710145/att_5787/v1/s71200_easy_book_en-US_en-US.pdf` |
| S8 | S7-1200 Programmable controller System Manual | 04/2012, A5E02486680-06, firmware V3.0 | `https://cache.industry.siemens.com/dl/files/465/36932465/att_106119/v1/s71200_system_manual_en-US_en-US.pdf` |
| S9 | S7-1200 Programmable controller, "Timers, counters and code blocks supported by CPU 1215C" | portal publication V20, "V4.7 release" | `tia:urp2Ni2UpoHXhbA9nhgm6A/v53Yi6OLn1toP7kOh3YytA` |
| S10 | S7-1200 G2 Programmable Logic Controller, CPU 1214C, "Blocks, timers, and counters" | portal publication V20 | `tia:tUwggX64ilkupyrzlTqJHw/MIBNOiySajkInORUgKTrug` |
| S11 | Extended instructions, "CREATE_DB: Create data block (S7-1200, S7-1500, S7-1200 G2)" | TIA Portal V21 | `tia:w2T~yjQ1OAvhCsh0htLcMA/kynDiCPjVqyYmaVOqOsC4g` |
| S12 | S7-1200 CPU firmware update V4.2, Product Information | 10/2016, A5E3537448-AF | `https://cache.industry.siemens.com/dl/files/628/109741628/att_902647/v1/s71200_firmware_update_cpu_v4_2_en-US_en-US.pdf` |
| S13 | Comparison list for programming languages S7-300/S7-400/S7-1200/S7-1500 | 05/2021 | `https://cache.industry.siemens.com/dl/files/648/109797648/att_1067421/v1/s71500_compare_table_en.pdf` |
| X1 | Rockwell FactoryTalk Optix help, "S7TCP driver" (secondary) | 1.4.1.7 | `https://help.optix.cloud.rockwellautomation.com/1.4.1.7/en/creating-projects/communication-driver/S7TCP-driver.html` |

## R2: Data types in a data block with standard access on S7-1500

### Answer

No Siemens source that the agent read refuses any of the five types in a block with standard access.
No source states the permission for a global data block directly. The evidence comes from function blocks with standard access and from the lists of differences between the two access types.

| Type | Answer | Confidence |
|---|---|---|
| `WSTRING` | Permitted. S2 permits it in every section of a standard FB except `InOut`. The instance DB of that FB holds the `Static` section. | medium |
| `LTIME` | Permitted by implication. S1 marks "Standard Data types" with X in the `Static` section of a function block on S7-1500, with no access condition. | medium |
| `LTOD` | Permitted by implication, on the same evidence as `LTIME`. | medium |
| `LDT` | Probably permitted. S1 marks the column "ARRAY STRUCT STRING / WSTRING DT" with X in `Static`. The column header does not name `LDT`. | low |
| `DTL` | Probably permitted, on the same evidence as `LDT`. X1 lists `DTL` for a driver that reads only blocks with standard access. | low |

### Evidence

- S2, section "Transferring a WSTRING for parameter supply".
  > For function blocks (FB) with "standard" access, operands of the data type WSTRING can be declared as parameters in all sections of the block interface except in the section "InOut".
- S1, table "Valid data types in the block interface in S7-1500", row "Function block / Static". The column "Standard Data types" holds "X". The column "ARRAY STRUCT STRING / WSTRING DT" holds "X". The row has no footnote.
- S1, footnote (1). It applies only to the cells that carry "(1)". The `Static` row does not carry "(1)".
  > WSTRING is only permitted in blocks with optimized access in these sections.
- S3, section "Data blocks with standard access". S3 names differences in structure, retentivity, and addressing. It names no data type.
  > Data blocks with standard access have a fixed structure. In the declaration, the data elements are assigned both a symbolic name and a fixed address within the block.
- S5, section 2.6, Table 2-5 "Difference: Optimized and non-optimized data block" names the offset and the retentivity. Table 2-6 "Access types" names the access types. Neither table names a data type.
- S5, section 2.6 gives the title of entry 67655611.
  > What types of access are available in STEP 7 (TIA Portal) to access data values in blocks and what should you watch out for with the differences between the types?
- S4 lists `LTIME`, `LTOD`, and `LDT` for S7-1500 only, and `DTL` and `WSTRING` for S7-1200 and S7-1500. It states no access condition.
- X1 (secondary): "The driver supports only non-optimized structures." Its type table lists `LTIME`, `LTIME_OF_DAY`, `DTL`, `WSTRING`, and `WCHAR`. It does not list `LDT`.

### Impact on the repo

- Section 10 of the fix plan says "New ADR if a type is refused". No source refuses a type, so no new ADR follows from R2 now.
- `data-model.md` §7 marks all five types "yes" on `S7_1500`. The research does not contradict that row.
- The S7 layout of these types in a block with standard access stays a point of R1. R1 checks offsets and sizes in TIA Portal.

## R3: `WCHAR` and `WSTRING` on S7-1200

### Answer

- The current documentation states that S7-1200 supports `WCHAR` and `WSTRING`. Confidence: high.
- Siemens documents for firmware V3.0 (S8) and V4.1 (S7) do not list `WChar` or `WString`. Thus the support starts after firmware V4.1. Confidence: medium.
- The agent found no primary source that names the first firmware or the first TIA Portal version. Secondary web pages name V4.0, V4.2, and V4.4. They disagree. Result: not found.
- The 12/2018 guideline (S5) does not contradict the current documentation. Its caption "only S7-1500" on Table 2-13 repeats the caption of the time table above it. This conclusion is an inference. Confidence: low.

### Evidence

- S4, table "Character strings", columns "S7-300/400 | S7-1200 | S7-1500". The rows are "WCHAR | - | X | X" and "WSTRING | - | X | X". S4 also has this note.
  > Depending on the CPU version, the actually valid data types can deviate slightly from the table.
- S6, table "Character and String data types". The rows are "WChar | 16 bits | 16#0000 to 16#FFFF" and "WString | n+ 2 words | n = (0 to 65534 words)". S6 also has this sentence.
  > You can also create string variables by selecting data type "String" or "WString" in the block interface editors for OB, FC, FB, and DB.
- S6 says that the S7-1200 accepts up to 65534 words in a `WString`. S2 limits a `WSTRING` to 16382 characters. The two Siemens documents disagree.
  > You can declare a length of up to 16382 characters (WSTRING[16382]).
- S1, footnote (1) for S7-1200. It applies to constants, function parameters, and the `InOut` section of a function block.
  > WSTRING is only permitted in blocks with optimized access in these sections.
- S7, Table 4-3 "Data types supported by the S7-1200", row "Character and string data types". The table has no `WChar` and no `WString`. S7 covers "S7-1200 CPU firmware release V4.1".
  > Char is an 8-bit single character. String is a variable-length string of up to 254 characters.
- S8 (firmware V3.0) contains no occurrence of `WString` or `WChar`.
- S12 lists the key features of firmware V4.2. It does not name a new data type.
- S5, section 2.8.4. The table below the sentence has the caption "Table 2-13: Time data types (only S7-1500)". Section 2.8.3 has "Table 2-12: Time data types (only S7-1500)" for `LTime` and `LTIME_OF_DAY`.
  > With the help of the data types WCHAR and WSTRING Unicode characters can be processed.
- S5, Table 4-1 and Table 4-2 do not list `WCHAR` or `WSTRING`.
- S13 marks `WCHAR` and `WSTRING` with two of the three columns. The text extraction of the PDF does not show which two columns.

### Queries without a primary answer

- Portal: `WSTRING data type S7-1200 firmware`.
- Portal: `S7-1200 WString WChar data type firmware V4`.
- Portal: `S7-1200 new features firmware V4.2 WString data type`.
- Web: `S7-1200 firmware V4.2 new features WString WChar data types STEP 7 V14`.
- Web: `"S7-1200" WSTRING WCHAR supported "firmware V4" OR "FW V4" TIA Portal version introduced Siemens`.
- The agent could not read the S7-1200 System Manual editions 03/2014 (V4.0) and later in PDF. `support.industry.siemens.com` answered `403`.

### Impact on the repo

- `data-model.md` §7 marks `WSTRING` and `WCHAR` "yes" on `S7_1200`. The current documentation supports that row. No change follows for the current firmware.
- A CPU with firmware V4.1 or earlier cannot hold these types. The matrix has no firmware dimension on S7. A firmware rule needs an ADR and a change of `data-model.md` §7, like ADR-0142 for TwinCAT 3.
- The length limit of `WSTRING` on S7-1200 is open. S6 gives 65534 words and S2 gives 16382 characters. The `StringLength` table of `domain-model.md` §2.1 is the place for that limit.

## R4: Data block numbers on S7-1200

### Answer

| Firmware | Data block numbers | Source | Confidence |
|---|---|---|---|
| V3.0 and earlier | 1 to 65535 | S8 (04/2012) | high |
| V4.0 to V4.6 | not found | none | - |
| V4.7 (current S7-1200) | 1 to 59999 | S9 | high |
| S7-1200 G2 | 1 to 59999 | S10 | high |
| `CREATE_DB` at run time (S7-1200, S7-1500, S7-1200 G2) | 60000 to 60999 | S11 | high |

The repo uses at most 59 999 on `S7_1200` and `S7_1500` (ADR-0140 (a)). That value equals the current S7-1200 manual and the S7-1200 G2 manual.
ADR-0140 read the S7-1200 value "only in a secondary copy". S9 and S10 are primary Siemens sources for the same value.

### Evidence

- S9, row "Address range for FBs, FCs, and DBs". The CPU 1214C page of the same publication has the same row.
  > FB and FC: 1 to 65535 (such as FB 1 to FB 65535) DB: 1 to 59999
- S6 and S9 come from one publication. Its page "New features" says: "The following features are new in the V4.7 release".
- S10, row "Address range for DBs": "1 to 59999".
- S8, technical specifications, row "Address range for FBs, FCs, and DBs": "1 to 65535 (such as FB 1 to FB 65535)".
- S8, section on the user program: "Data blocks (DBs) store data that can be used by the program blocks. Valid DB numbers range from 1 to 65535."
- S8 covers "S7-1200 CPU firmware release V3.0".
- S11, parameter `LOW_LIMIT`: "The smallest possible number is 60000." Parameter `UP_LIMIT`: "(largest possible DB number: 60999)".

### Impact on the repo

- ADR-0140 (a) stays valid for the current S7-1200 firmware and for S7-1200 G2. S11 supports the reason of ADR-0140 Option B: `CREATE_DB` owns 60000 to 60999.
- A CPU with firmware V3.0 or earlier accepts a data block up to 65535. ADR-0140 names this risk. A limit per firmware needs a new ADR with Option C of ADR-0140.
- The ADR of 7.2 of the fix plan is ADR-0140. It can replace the words "The review read it only in a secondary copy" through an "Amended by" note, if the owner wants that.

## Open points

1. R2: Siemens entry 67655611 stays unread. A person with a browser session can read it and compare it with this report.
2. R2: No source names `LDT` and `DTL` in a block with standard access. A TIA Portal check closes this point. The check can run with R1: one global DB with standard access on S7-1500 that holds the five types.
3. R3: The first firmware and the first TIA Portal version for `WCHAR` and `WSTRING` on S7-1200 stay unknown. The editions of the S7-1200 System Manual from 03/2014 to 2022 can answer it.
4. R3: S6 and S2 disagree on the maximum length of `WSTRING` on S7-1200: 65534 words or 16382 characters.
5. R4: The range for firmware V4.0 to V4.6 stays unknown. The same manual editions as in point 3 can answer it.
6. Outside the scope: S10 lists `IEC_LTIMER` for S7-1200 G2. The V21 data type help groups S7-1200 G2 with S7-1500. `data-model.md` §7 has one column `S7_1200` and marks `LTIME` "no". The owner can decide if S7-1200 G2 needs its own platform.
