# PLC data types in CODESYS V3.5 and TwinCAT 3.1
## Byte layout, bit meaning and decoding reference

**Scope.** This reference covers native PLC variables in the CODESYS V3.5 service-pack family through SP22 and TwinCAT 3.1 after build 4024, with 4024 included as a compatibility baseline. The primary profiles are CODESYS V3.5 SP22 and TwinCAT 3.1 build 4026. It covers the elementary type inventories of both vendors, strings, user-defined types, indirect and generic types, and selected system/library types frequently confused with elementary types. Completeness here means every standard type spelling in the two vendor inventories, plus the long date/time spellings on their dedicated pages. The reference also covers every DUT category in the TwinCAT inventory. Section 8 gives selected declarations from optional products and libraries. Their complete catalogs depend on the installed packages and target.

**How to use this reference.** For a raw memory read, identify the declared type, target byte order and compiler profile first. For an aggregate, obtain its layout from the same application build that produced the bytes. An ADS symbol, a CODESYS exported symbol, an OPC UA value and a Modbus register mapping can present the same PLC value through different representations.

| Section | Look up |
|---|---|
| [1. Target and version](#1-target-and-version) | Byte order, bit numbering, feature availability |
| [2. Booleans and integers](#2-booleans-and-integers) | Flags, bit strings, signed and unsigned integers |
| [3. Floating point](#3-floating-point) | IEEE 754 fields, ranges, special values |
| [4. Time and date](#4-time-and-date) | All eight duration, date and clock types, epochs and examples |
| [5. Strings and characters](#5-strings-and-characters) | Allocation, terminators, encodings and character literals |
| [6. Arrays and user-defined types](#6-arrays-and-user-defined-types) | Strides, padding, unions, enumerations and subranges |
| [7. Addresses and runtime types](#7-addresses-and-runtime-types) | Native integers, pointers, references, interfaces and generic inputs |
| [8. Selected system and library types](#8-selected-system-and-library-types) | Version fields, vectors, diagnostic codes and alternative clocks |
| [9. Checking a compiled layout](#9-checking-a-compiled-layout) | ST probe, expected results and symbol metadata |

## 1. Target and version

One byte means eight bits. Byte offsets start at zero and increase with memory address. When a byte is read as unsigned, bit 0 has weight 1 and bit 7 has weight 128. In a `SINT`, bit 7 contributes −128. In a floating-point value the sign, exponent and fraction fields have separate meanings. An example such as `78 56 34 12` lists hexadecimal bytes in **ascending address order**. Unless marked otherwise, examples assume little-endian (LE) storage.

| Property | CODESYS V3.5 | TwinCAT 3.1 |
|---|---|---|
| Native byte order | Target property. `__SYSTEM.Constants.bLittleEndian` identifies it. | Beckhoff documents little-endian PLC host order. |
| Native address size | 4 or 8 bytes on the documented 32-/64-bit targets. | 4 or 8 bytes according to the PLC target architecture. |
| Aggregate alignment | Selected target, declaration and packing attributes determine offsets. | Default system alignment is 8 bytes. Smaller members and aggregates can have smaller alignment requirements. |
| `LDATE`, `LDATE_AND_TIME`, `LTIME_OF_DAY` | Introduced as language features in V3.5 SP16. | Available from 3.1.4026.0. Build 4024 already has `LTIME` for durations. |
| `UTF8#'…'` literal | Compiler 3.5.18.0 or later. | Build 4026 or later. An encoded byte buffer can also be produced with a conversion function. |
| Object-size operator | `XSIZEOF` recommended from compiler 3.5.16.0. Use `SIZEOF` with earlier compilers. | `XSIZEOF` from build 4026. Use `SIZEOF` on 4024. |

**Scalar byte map.** For an unsigned value with bytes `b[0]…b[n−1]`, the LE value is `Σ b[i] × 256^i`. Big-endian (BE) uses `Σ b[i] × 256^(n−1−i)`. For example, `DWORD#16#12345678` occupies `78 56 34 12` in LE and `12 34 56 78` in BE. A byte swap changes byte order only. It does not reverse bits within a byte, reorder members in a structure or convert the measurement unit.

| Offset in an LE scalar | Scalar bits | Applies to |
|---:|---|---|
| 0 | 7…0 | All widths |
| 1 | 15…8 | 2, 4 and 8 bytes |
| 2, 3 | 23…16, 31…24 | 4 and 8 bytes |
| 4, 5, 6, 7 | 39…32, 47…40, 55…48, 63…56 | 8 bytes |

Sources: [CODESYS type inventory](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_reference_datatypes.html) · [TwinCAT type inventory](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529388939.html) · [CODESYS system properties](https://content.helpme-codesys.com/en/LibDevSummary/friends.html) · [TwinCAT byte order](https://infosys.beckhoff.com/content/1033/tcplclib_tc2_utilities/35311883.html) · [CODESYS SP16 feature notes](https://api-de.codesys.com/fileadmin/user_upload/CODESYS_Group/Ecosystem/Up-to-Date/Releases-Lifecycle/Release-Updates/Features-Improvements/features-and-improvements-V35SP16-en.pdf) · [TwinCAT 4026 features](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/13729845515.html).

## 2. Booleans and integers

### Logical values and bit strings

| Type | Storage | Meaning and example |
|---|---:|---|
| `BOOL` | 1 byte for an ordinary variable | Canonical bytes `00` = `FALSE` and `01` = `TRUE`. Other byte values are outside the documented logical encoding. Eight ordinary `BOOL` array elements occupy 8 bytes. |
| `BIT` | 1 bit in a permitted declaration | A named `FALSE`/`TRUE` field in a structure or function block. Consecutive declarations share bytes. Eight consecutive flags need one payload byte, nine need two, before any aggregate padding. Arrays of `BIT` and pointers/references to individual `BIT`s are invalid. |
| `BYTE` | 1 byte | Bits 0…7. Example `81` sets bits 7 and 0. |
| `WORD` | 2 bytes | Bits 0…15. `WORD#16#8001` → `01 80`. |
| `DWORD` | 4 bytes | Bits 0…31. `DWORD#16#80000001` → `01 00 00 80`. |
| `LWORD` | 8 bytes | Bits 0…63. `LWORD#16#8000000000000001` → `01 00 00 00 00 00 00 80`. |

For a numeric bit string, bit `k` has mask `2^k`. On an LE target it lies in byte `k DIV 8` at bit `k MOD 8`. Thus bit 12 in a `WORD` uses mask `16#10` at byte offset 1. For a **named** `BIT` member, use its compiled bit offset. A run of flags may start partway through a larger mapped object. A byte holding several flags cannot be replaced to update one flag without accounting for its neighbours.

A mapped declaration such as `xReady AT %IX0.3 : BOOL;` refers to one process-image bit. The declaration does not allocate a separate free-standing flag byte in that image. `%I`, `%Q` and `%M` identify input, output and memory areas. The device's byte- or word-oriented address conventions determine how a position in one area relates to a byte offset.

Sources: [CODESYS BOOL](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_bool.html) · [CODESYS BIT](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_bit.html) · [TwinCAT BIT](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529442699.html) · [CODESYS AT declarations](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_at_declaration.html).

### Fixed-width integers

Unsigned integers use every bit for magnitude. For a signed integer of width `w`, first read the bytes as an unsigned integer `u`. If bit `w−1` is set, the two's-complement value is `u − 2^w`. Otherwise it is `u`. The sign is part of the numerical encoding. Clearing the sign bit does not recover the absolute value of a negative integer.

| Type | Bytes | Scalar bits | Inclusive range | Example value → LE bytes |
|---|---:|---|---|---|
| `SINT` | 1 | Sign 7, low bits 6…0 | −128…127 | −2 → `FE` |
| `USINT` | 1 | 7…0 unsigned | 0…255 | 254 → `FE` |
| `INT` | 2 | Sign 15, low bits 14…0 | −32,768…32,767 | −300 → `D4 FE` |
| `UINT` | 2 | 15…0 unsigned | 0…65,535 | 300 → `2C 01` |
| `DINT` | 4 | Sign 31, low bits 30…0 | −2,147,483,648…2,147,483,647 | −300 → `D4 FE FF FF` |
| `UDINT` | 4 | 31…0 unsigned | 0…4,294,967,295 | 70,000 → `70 11 01 00` |
| `LINT` | 8 | Sign 63, low bits 62…0 | −9,223,372,036,854,775,808…9,223,372,036,854,775,807 | −2 → `FE FF FF FF FF FF FF FF` |
| `ULINT` | 8 | 63…0 unsigned | 0…18,446,744,073,709,551,615 | 4,294,967,296 → `00 00 00 00 01 00 00 00` |

A signed scalar of width `w` has most-significant-bit weight `−2^(w−1)`. The lower `w−1` bits retain positive binary weights. For example, `80` represents −128 as `SINT` and 128 as `USINT`.

The same `FF FF` bytes can represent `INT#-1`, `UINT#65535` or a `WORD` with sixteen set bits. A type declaration supplies that interpretation. A raw integer carries neither engineering units nor a decimal scale. For example, the application must specify whether a `UINT` value of 253 means 253 units or 25.3 °C.

Sources: [CODESYS integer widths and ranges](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_integer.html) · [TwinCAT integer widths and ranges](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529399691.html).

## 3. Floating point

| Type | Bytes | Sign `s` | Exponent `E` | Stored fraction `F` | Bias | Significand precision |
|---|---:|---:|---|---|---:|---:|
| `REAL`, binary32 | 4 | Bit 31 | Bits 30…23, 8 bits | Bits 22…0, 23 bits | 127 | 24 bits for normal numbers |
| `LREAL`, binary64 | 8 | Bit 63 | Bits 62…52, 11 bits | Bits 51…0, 52 bits | 1023 | 53 bits for normal numbers |

The bit positions refer to the assembled unsigned bit pattern. For a normal finite number with `f` stored fraction bits, its value is `(−1)^s × (1 + F/2^f) × 2^(E−bias)`. The leading 1 is implicit. Values such as decimal 0.1 generally round to the nearest representable binary fraction.

| Exponent field | Fraction field | Meaning |
|---|---|---|
| All zero | Zero | Positive or negative zero, according to the sign bit |
| All zero | Nonzero | Subnormal, `(−1)^s × (F/2^f) × 2^(1−bias)` |
| Neither all zero nor all one | Any | Normal finite number |
| All one | Zero | Positive or negative infinity |
| All one | Nonzero | NaN. The remaining fraction bits may carry a payload. |

| Property | `REAL` | `LREAL` with binary64 support |
|---|---:|---:|
| Largest finite magnitude | ≈ 3.40282347 × 10³⁸ | ≈ 1.7976931348623157 × 10³⁰⁸ |
| Smallest positive normal | 2⁻¹²⁶ | 2⁻¹⁰²² |
| Smallest positive subnormal | 2⁻¹⁴⁹ | 2⁻¹⁰⁷⁴ |
| `1.0` in memory, LE | `00 00 80 3F` | `00 00 00 00 00 00 F0 3F` |
| `−2.5` in memory, LE | `00 00 20 C0` | `00 00 00 00 00 00 04 C0` |

CODESYS documents targets whose compiler converts `LREAL` to `REAL`. Confirm the target's floating-point support and compiled symbol size before binding an external binary64 reader. The existence of IEEE special bit patterns also does not establish how a particular PLC handles arithmetic exceptions or subnormal *operations*.

Sources: [CODESYS floating-point types and target exception](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_real.html) · [TwinCAT floating-point types](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529405067.html).

## 4. Time and date

Each row below describes **one binary integer counter**. There are no separate bytes for year, month, day, hours or milliseconds. Decode the entire counter with the width and signedness in the table, then apply its unit. The epoch for the native calendar types is 1970-01-01 00:00:00. Time-zone offset, daylight-saving state and zone name occupy zero bits. Record the time basis used by the producer, especially when it has converted UTC to local time before assignment.

| Type and equivalent keyword | Bytes | Meaning of the stored integer | Valid value domain |
|---|---:|---|---|
| `TIME` | 4 | Unsigned milliseconds of duration | 0…4,294,967,295 ms, ending at 49 d 17 h 2 min 47.295 s |
| `LTIME` | 8 | Unsigned nanoseconds of duration | 0…2⁶⁴−1 ns, ending at 213,503 d 23 h 34 min 33.709551615 s |
| `DATE` | 4 | Unsigned seconds from epoch. A date literal identifies the start of a calendar day. | 1970-01-01…2106-02-07, displayed at day granularity |
| `DATE_AND_TIME` / `DT` | 4 | Unsigned seconds from epoch | 1970-01-01 00:00:00…2106-02-07 06:28:15 |
| `TIME_OF_DAY` / `TOD` | 4 | Unsigned milliseconds from midnight | 0…86,399,999, ending at 23:59:59.999 |
| `LDATE` | 8 | Nanoseconds from epoch, interpreted as signed two's complement. Date literals select midnight. | 1677-09-22…2262-04-11 |
| `LDATE_AND_TIME` / `LDT` | 8 | Signed two's-complement nanoseconds from epoch | 1677-09-21 00:12:43.145224192…2262-04-11 23:47:16.854775807 |
| `LTIME_OF_DAY` / `LTOD` | 8 | Nonnegative nanoseconds from midnight | 0…86,399,999,999,999, ending at 23:59:59.999999999 |

**Date versus counter width.** Successive `DATE` midnights differ by 86,400 raw units. Successive `LDATE` midnights differ by 86,400,000,000,000. Date-only display can conceal a time component after a raw reinterpretation. To encode a canonical date from a calendar value, encode midnight. If the application requires canonical input, validate the midnight multiple. For `TOD` and `LTOD`, reject counts at or beyond one day instead of silently wrapping malformed data.

**Signed epoch, unsigned duration.** The long calendar documentation sometimes names the 64-bit container `LWORD` or `ULINT`. Its calendar range extends before 1970. For `LDT`, `16#8000000000000000` is the earliest instant and `16#7FFFFFFFFFFFFFFF` is the latest. The same all-one pattern means −1 ns for `LDT` and 2⁶⁴−1 ns for `LTIME`. Store signed nanoseconds in an integer when exact sub-microsecond precision matters.

| Value | Counter in decimal | Bytes, LE |
|---|---:|---|
| `T#1s250ms` | 1,250 ms | `E2 04 00 00` |
| `LTIME#1us` | 1,000 ns | `E8 03 00 00 00 00 00 00` |
| `D#1970-01-02` | 86,400 s | `80 51 01 00` |
| `DT#1970-01-01-00:00:01` | 1 s | `01 00 00 00` |
| `TOD#00:00:01.234` | 1,234 ms | `D2 04 00 00` |
| `LD#1970-01-02` | 86,400,000,000,000 ns | `00 00 4F 91 94 4E 00 00` |
| `LDT#1969-12-31-23:59:59.999999999` | −1 ns | `FF FF FF FF FF FF FF FF` |
| `LTOD#00:00:00.000000001` | 1 ns | `01 00 00 00 00 00 00 00` |

The shorter prefixes `T#`, `D#` and `LD#` introduce literals. Declare variables as `TIME`, `DATE` and `LDATE`. By contrast, `DT`, `TOD`, `LDT` and `LTOD` are also type names. A 1 ns *representation* does not imply a 1 ns clock or task resolution.

Sources: [CODESYS durations](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_time.html) · [CODESYS calendar types](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_date_and_time_of_day.html) · [CODESYS clock conventions](https://content.helpme-codesys.com/en/LibDevSummary/date_time.html) · [TwinCAT durations](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/12189021579.html) · [TwinCAT calendar types and minimum build](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529415819.html).

## 5. Strings and characters

| Declaration | Reserved bytes | Layout from offset 0 | Logical end |
|---|---:|---|---|
| `STRING(n)` | `n + 1` | Encoded bytes in increasing address order, followed by a zero byte and any unused capacity | First `00` |
| `STRING` | 81 | Equivalent to `STRING(80)` | First `00` |
| `WSTRING(n)` | `2 × (n + 1)` | Up to `n` UCS-2 code units in target byte order, followed by a zero word and any unused capacity | First aligned `0000` code unit |
| `WSTRING` | 162 | Equivalent to `WSTRING(80)` | First zero word |

Here `n` is the declared capacity excluding the terminator. Neither representation stores a length prefix. In a raw read, search only within the declared allocation. A missing terminator is a malformed or incomplete value. Bytes after the terminator can contain previous contents, including after some reset/reinitialization paths. Logical equality uses the text through its terminator.

| Declared value | Bytes at ascending offsets | How to read it |
|---|---|---|
| `STRING(4)` containing `AB` | `41 42 00 xx xx` | 5 bytes allocated, 2 bytes of text. `xx` denotes unspecified tail contents. |
| `STRING(2)` containing `é`, Latin-1 | `E9 00 xx` | One character uses one payload byte. |
| `STRING(2)` containing `é`, UTF-8 | `C3 A9 00` | The same character fills both payload bytes. |
| `STRING(4)` containing `😀`, UTF-8 | `F0 9F 98 80 00` | One Unicode scalar uses the full four-byte capacity. |
| `WSTRING(2)` containing `AŁ`, LE | `41 00 41 01 00 00` | Read words `0041`, `0141`, `0000`. A byte-wise null scan would stop inside the first character. |
| `WSTRING(2)` containing `AŁ`, BE | `00 41 01 41 00 00` | Code-unit order is unchanged. Each word uses BE byte order. |

**Encoding selection.** Both vendors document Latin-1 and UTF-8 interpretations of `STRING`. In CODESYS check the project compile option *UTF8 Encoding for STRING* and any explicit literal prefix. TwinCAT supports `{attribute 'TcEncoding' := 'UTF-8'}` and, from build 4026, the `UTF8#` literal prefix. The declaration, producer and reader must agree about encoding. Copying Latin-1 bytes into a UTF-8-labelled buffer does not transcode them. Capacity is measured in bytes even when a character takes two to four bytes. Older string library functions can impose their own processing limits, commonly 255, independently of the declaration.

**Character values.** Both vendors describe `WSTRING` as UCS-2, covering U+0000…U+D7FF and U+E000…U+FFFF, with U+0000 reserved here for termination. Supplementary characters such as 😀 require another representation, such as a UTF-8 `STRING`. String indexing is zero-based. For a `STRING`, `s[0]` reads its first byte (`BYTE`). For a `WSTRING`, `ws[0]` reads its first code unit (`WORD`). For UTF-8 `'é'`, indices 0 and 1 give `C3` and `A9`, respectively. `STRING(1)` occupies 2 bytes and `WSTRING(1)` occupies 4 bytes.

**A code point in an integer.** The documented `UCHAR#'Ł'` literal has base type `UDINT`, value U+0141 = 321, and bytes `41 01 00 00` when stored in a `UDINT` on LE. TwinCAT documents this literal from build 4026. Neither vendor's standard-type inventory lists standalone `CHAR` or `WCHAR` PLC scalar declarations. Distinguish library aliases and language-binding names from these PLC types.

Sources: [CODESYS STRING](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_string.html) · [CODESYS UTF-8 literals](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_operands_constant_string_utf8.html) · [CODESYS WSTRING](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_wstring.html) · [CODESYS character literal](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_operands_constant_character.html) · [TwinCAT STRING](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529410443.html) · [TwinCAT WSTRING](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529437323.html) · [TwinCAT encoding attribute](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/5873680907.html) · [TwinCAT character literal](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/17784854155.html).

## 6. Arrays and user-defined types

### Fixed arrays, open arrays and indexing

`ARRAY[L..U] OF T` has `U − L + 1` elements. Its ordinary inline payload is the element count multiplied by the compiled element stride. For a fixed array of ordinary scalar elements the stride is the scalar size in bytes. Element `i` starts at `base + (i − L) × stride`. The bounds belong to the declaration or symbol metadata. An inline fixed array has no per-instance length prefix.

| Declaration | Size | Consequence |
|---|---:|---|
| `ARRAY[-2..2] OF INT` | 10 bytes | Indices −2, −1, 0, 1, 2 start at offsets 0, 2, 4, 6, 8. |
| `ARRAY[1..8] OF BOOL` | 8 bytes | Each element is a separate canonical Boolean byte. |
| `ARRAY[1..3] OF STRING(4)` | 15 bytes | Each slot occupies five bytes including its own terminator capacity. |
| `ARRAY[1..3] OF ST_Sample` | 3 × compiled structure stride | Each element includes the structure's alignment padding. See the worked layout below. |
| `ARRAY[*,*] OF INT` as `VAR_IN_OUT` | Actual caller's element count × 2 for the underlying fixed array | An open-array formal receives the caller's bounds. Read `LOWER_BOUND` and `UPPER_BOUND` for each dimension. The call descriptor has a compiler-specific layout. |

**Multiple dimensions.** A flat initializer and physical memory offsets answer different questions. The vendors' published three-dimensional initializer examples assign the **second** initializer to different indices. Here is the comparison for their `ARRAY[1..2,3..4,5..6] OF INT` example, with initial values 10, 20, …, 80.

| Index | Value in the CODESYS help example | Value in the TwinCAT help example |
|---|---:|---:|
| `[1,3,5]` | 10 | 10 |
| `[2,3,5]` | 20 | 50 |
| `[1,3,6]` | 50 | 20 |
| `[2,4,6]` | 80 | 80 |

For a binary reader, use measured or exported byte strides. In two dimensions the general formula is `offset(i,j) = (i−L1) × stride1 + (j−L2) × stride2`. For a 2 × 3 matrix of `UINT`, strides `(6,2)` imply that the rightmost index varies fastest. Strides `(2,4)` imply that the leftmost varies fastest. Section 9 measures these directly and independently reads the initializer values. A nested declaration `ARRAY[1..2] OF ARRAY[1..3] OF UINT` explicitly stores two inner arrays. Each inner array occupies 6 bytes and each of its elements occupies 2 bytes.

Sources: [CODESYS array declarations and initializer examples](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_array.html) · [TwinCAT fixed-array initializer examples](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/8825253771.html) · [TwinCAT nested arrays](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/8825255691.html).

### Structures, alignment and padding

A `STRUCT` groups fields in declaration order. A field's storage requirement and its alignment requirement are separate. For ordinary fixed-size members, advance to the next offset allowed by the member's effective alignment, place the member, then advance by its size. The compiler may add tail padding so the next array element begins at a suitable boundary. Nested structures retain their own internal layout. Structure extension with `EXTENDS` also requires the compiled base and derived layout.

```iecst
TYPE ST_Sample :
STRUCT
    byState  : BYTE;
    udiCount : UDINT;
    uiFlags  : UINT;
    xValid   : BOOL;
END_STRUCT
END_TYPE
```

| Field | Natural layout with 4-byte alignment for `UDINT` | Same declaration with `{attribute 'pack_mode' := '1'}` |
|---|---|---|
| `byState` | Byte 0 | Byte 0 |
| `udiCount` | Bytes 4…7 | Bytes 1…4 |
| `uiFlags` | Bytes 8…9 | Bytes 5…6 |
| `xValid` | Byte 10 | Byte 7 |
| Padding | Bytes 1…3 and 11 | None |
| **Total / array stride** | **12 bytes** | **8 bytes** |

With `byState = 2`, `udiCount = 16#12345678`, `uiFlags = 16#8001` and `xValid = TRUE`, the natural LE image is `02 xx xx xx 78 56 34 12 01 80 01 xx`. The byte-packed image is `02 78 56 34 12 01 80 01`. Padding has no domain meaning. A comparison or checksum over the whole natural image can differ even when every PLC field has the same value.

| Explicit `pack_mode` | Effect on ordinary numeric members in these vendors' packing tables |
|---:|---|
| 0 | Documented byte-addressed placement without gaps for the ordinary fields shown. Omitting the attribute selects the target default. |
| 1 | Byte alignment. Even a `UDINT` can start at an odd offset. |
| 2 | Alignment capped at 2 bytes. A `BYTE` still requires only 1. |
| 4 | Alignment capped at 4 bytes. An 8-byte scalar can start on a 4-byte boundary. |
| 8 | Allows alignment up to 8 bytes. Does not force each field or every structure to occupy a multiple of 8. |

Place the attribute above the DUT declaration. For CODESYS symbol export, also record any *Compatibility layout* setting. The published symbol view can require separate attention when combined with explicit packing.

Sources: [CODESYS structures](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_structure.html) · [CODESYS pack modes and byte maps](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_pragma_attribute_pack_mode.html) · [TwinCAT default alignment](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/3539428491.html) · [TwinCAT pack modes](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529746059.html).

### Aliases, subranges, enumerations and unions

| Form | Storage rule | Example and decoding |
|---|---|---|
| Alias | Same size, layout and byte order as the aliased type. | `TYPE T_BatchId : UDINT; END_TYPE` occupies 4 bytes. The alias gives a process meaning to the number. |
| Subrange | Same integer representation as the base type. Subrange bounds occupy no bytes in each instance. | `UINT (0..1000)` occupies 2 bytes. `E8 03` is 1000. `E9 03` decodes numerically to 1001, outside the declared range. |
| Enumeration | The base integer type stores the enumerator's value. Default base is `INT`, 2 bytes. An explicit base sets width and signedness. | `TYPE E_Mode : (Idle := 0, Run := 10, Fault := 255) USINT; END_TYPE` uses 1 byte. `0A` is `Run`. `01` is an unassigned numeric code. |
| `UNION` | All alternatives begin at offset 0. Reserve enough space for the largest member under the target's alignment rules. Obtain the full object size and array stride from the compiled DUT. | A union of `DWORD`, `REAL` and `ARRAY[0..3] OF BYTE` normally occupies 4 bytes. LE bytes `00 00 80 3F` mean `REAL#1.0` or `DWORD#16#3F800000`. |

An enum name, a subrange bound and a union's active alternative are schema information. A union carries no implicit tag. If a protocol selects a union member, obtain its discriminator from the surrounding record or message definition. Writing a smaller member can leave bytes belonging to a larger alternative untouched. A `{attribute 'strict'}` enum restricts operations at compilation. It adds no run-time tag. An enum's declared initial value and the raw code zero can differ. An implicit subrange monitor such as `CheckRangeUnsigned` applies to PLC execution paths configured to use it. An external decoder can independently read `E9 03`, compare 1001 against the declared upper bound of 1000 and report the out-of-range value.

Sources: [CODESYS subranges and runtime checks](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_subint.html) · [CODESYS enumerations](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_enum.html) · [TwinCAT derived-type index, including aliases and subranges](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529388939.html) · [TwinCAT union layout and overlays](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529421195.html).

## 7. Addresses and runtime types

### Native integers

| Declaration | 32-bit PLC | 64-bit PLC | Meaning |
|---|---|---|---|
| `__XINT` | `DINT`, 4 bytes | `LINT`, 8 bytes | Signed native-width integer |
| `__UXINT` | `UDINT`, 4 bytes | `ULINT`, 8 bytes | Unsigned native-width integer |
| `__XWORD` | `DWORD`, 4 bytes | `LWORD`, 8 bytes | Native-width bit string |
| `PVOID`, TwinCAT | `UXINT`, 4 bytes | `UXINT`, 8 bytes | An untyped address value |

TwinCAT also documents the spellings `XINT`, `UXINT` and `XWORD`. CODESYS documents the double-underscore spellings. The width follows the target PLC, irrespective of the engineering workstation. The integer and byte rules in sections 1 and 2 apply after resolving the native type. For example, unsigned native value 1 is `01 00 00 00` on a 32-bit LE PLC and `01 00 00 00 00 00 00 00` on a 64-bit LE PLC.

Sources: [CODESYS native-width types](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_uxint_xword.html) · [TwinCAT native-width spellings and PVOID](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529448075.html).

### Indirection, object instances and generic arguments

| Construct | What occupies memory | What can be decoded |
|---|---|---|
| `POINTER TO T` | Native address, typically 4 or 8 bytes. The pointed-to `T` occupies separate storage. | `ADR(x)` obtains the address and `p^` accesses its target. Zero is a null pointer. Pointer indexing uses the element size. For `p : POINTER TO UINT`, `p[3]` reads a 2-byte value at an offset of 6 bytes. A pointer stored in a record changes that record's offsets when its width changes. |
| `REFERENCE TO T` | Compiler-managed address binding, normally pointer-sized. Source-level use implicitly accesses `T`. | `r REF= x` binds the reference. With a valid binding, `r := value` writes the referenced object. Use the containing symbol layout to determine the binding's slot. |
| `VAR_IN_OUT` | A parameter-passing convention providing access to caller storage. | The supplied object's type defines the data. A caller's `UINT` remains a 2-byte value regardless of the call ABI. |
| `INTERFACE` variable | A binding to an implementing function-block instance. The compiler determines representation. | Through the interface, access the declared methods/properties. Its in-memory handle has no portable field layout or persistent object identity. |
| `FUNCTION_BLOCK` instance | Per-instance inputs, outputs and internal state as applicable, alignment and compiler-managed fields. TwinCAT documents a virtual-method-table pointer as an example. | Use compiled symbols to identify published members. The sum of visible fields does not determine instance size. A timer `TON` is an FB instance, while its `PT`/`ET` values have type `TIME`. |
| `ANY` and generic families | Call descriptor containing the actual type class, a pointer to the value, and the actual size. | `typeclass` identifies the supplied type, `pvalue` is a `POINTER TO BYTE`, and `diSize` is a `DINT` byte count. Pointer width, enum width and ABI alignment determine descriptor offsets. |

### Generic families and the call descriptor

| Formal input | Actual arguments in the vendors' generic-input tables | Payload interpretation |
|---|---|---|
| `ANY` | Generic variable input. The CODESYS library guide also gives examples of structured, array and pointer arguments. | Use the actual type class and declaration. Passing an address value differs from passing the object it addresses. |
| `ANY_BIT` | `BYTE`, `WORD`, `DWORD`, `LWORD` | 1, 2, 4 or 8 bytes of bits. Both published input tables list these four types. Do not infer acceptance of `BOOL` or a packed `BIT` from the family name. |
| `ANY_INT` | The eight signed and unsigned fixed-width integer types in section 2 | Select signedness and width from the actual type. Native integer aliases resolve for the target. |
| `ANY_REAL` | `REAL`, `LREAL` | Decode the target's actual floating-point format. |
| `ANY_NUM` | `ANY_INT` and `ANY_REAL` families | The type class distinguishes integer from floating-point payloads of the same width. |
| `ANY_DATE` | `DATE`, `DT`, `TOD`, `LDATE`, `LDT`, `LTOD`, including full spellings where available | Apply section 4 to the concrete type. Duration types use a different interpretation. |
| `ANY_STRING` | `STRING`, `WSTRING` | Use the actual storage extent and the matching terminator size. |

For example, a `REAL` passed to `ANY_REAL` has a four-byte payload. Access it through the descriptor's `pvalue` field. `diSize` describes the supplied data in bytes. TwinCAT documents generic inputs on functions and methods and extends their use to function blocks in build 4026. Both development-system manuals specify variable arguments and exclude constants and properties at these inputs. CODESYS also mentions `ANY_DURATION` in its library guide, with an explicit footnote that it is unimplemented. It is absent from both vendors' supported generic-input lists.

**Descriptor field order.** The CODESYS Development System and TwinCAT PLC manuals illustrate `typeclass, pvalue, diSize`. The CODESYS Library Development Guidelines show `pValue, diSize, TypeClass` in their declaration of `AnyType`. For a 32-bit example with three four-byte fields, these sequences place the pointer at byte 4 and byte 0, respectively. These are illustrations of the consequences of the published declarations. Access the fields by name in PLC code. A host that needs raw offsets must obtain the exact compiled declaration, target pointer width and packing.

An address observed during one run can lose its meaning after relocation, a restart, a new download or an online change. TwinCAT 4026 adds symbol-dependent automatic adjustment of PLC pointers and references during online change. A host reading a symbol should reacquire its binding when the application changes.

Sources: [CODESYS library guide's AnyType declaration](https://content.helpme-codesys.com/en/LibDevSummary/any_type.html) · [CODESYS generic families and ANY_DURATION status](https://content.helpme-codesys.com/en/LibDevSummary/friends.html) · [CODESYS generic argument descriptor](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_any.html) · [TwinCAT generic argument descriptor](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529426571.html) · [TwinCAT references and online change](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529458827.html) · [TwinCAT instance alignment and implicit fields](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/3539428491.html).

## 8. Selected system and library types

### Compiler and system types

| Type | Size | Field or bit meaning |
|---|---:|---|
| `VERSION`, CODESYS project/library information | 8 bytes for the four documented `UINT` fields | Offsets 0…1 `uiMajor`, 2…3 `uiMinor`, 4…5 `uiServicePack`, 6…7 `uiPatch`. Version 3.5.22.0 in LE is `03 00 05 00 16 00 00 00`. |
| `__VECTOR[n] OF REAL` / `OF LREAL`, CODESYS | Logical payload is `n × element size`. Verify the compiled vector size and alignment for the target. | `n` is 1…8. Lane indices run from 0 to `n−1`. Each lane holds a floating-point value. Four binary32 lanes carry 16 bytes of numerical data. Native SIMD support depends on CPU and code generator. |
| `__SYSTEM.ExceptionCode`, CODESYS and TwinCAT | 4 bytes | A `UDINT`-based enum. The whole unsigned value identifies one exception. `00 00 00 00` means no exception, `10 00 00 00` means watchdog, and `FF FF FF FF` means unknown (LE). |
| `__SYSTEM.TYPE_CLASS`, CODESYS | 4 bytes in the published `DWORD`-based enumeration | Identifies the data type of an actual value. Examples include `TYPE_BOOL` = 0, `TYPE_INT` = 7, `TYPE_REAL` = 14 and `TYPE_ARRAY` = 26. The class does not encode a string capacity, an array's bounds or a DUT's layout. |
| `__SYSTEM.MEMORY_AREA`, CODESYS | The published enumeration has the default `INT` base, 2 bytes | Codes −1, 0, 1, 2, 3, 4 and 5 designate unknown, marker, input, output, retain, global and local memory respectively. This describes an area category. The separate `Area` member of `VAR_INFO` is a target-specific identifier. |
| `__SYSTEM.VAR_INFO` | Compiler-defined structure | `ByteOffset` is relative to the global area, instance or stack frame as applicable. `BitSize` gives the variable's size in bits. For arrays, `NumElements` is the element count and `ElemBitSize` is the base element size in bits. `BitAddress` is meaningful for `%I`, `%Q` and `%M`. Consult the selected compiler's field names and widths when reading this record. |

Sources: [CODESYS exception code declaration](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_operator_try_catch_finally_endtry.html) · [CODESYS TYPE_CLASS](https://content.helpme-codesys.com/en/LibDevSummary/type_class.html) · [CODESYS MEMORY_AREA](https://content.helpme-codesys.com/en/LibDevSummary/memory_area.html) · [CODESYS variable information](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_operator_varinfo.html) · [CODESYS VERSION](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_version.html) · [CODESYS vector syntax and target support](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_data_type_vector.html) · [TwinCAT exception code declaration](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/6753940235.html) · [TwinCAT variable information](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/3527777675.html).

### Clock and address types encountered at library boundaries

| Library/API type | Bytes | How to decode the data |
|---|---:|---|
| CODESYS `SysTime` as used by `SysTimeRtcHighResGet` | 8 | Unsigned milliseconds since 1970-01-01 00:00:00 UTC for this call. Other SysTime calls can use different units and time bases. |
| CODESYS `RTS_SYSTIMEDATE` (`SysTimeRtc` library) | 18 in the documented nine-`UINT` layout | Offsets 0, 2, 4, 6, 8, 10, 12, 14 and 16 hold year, month, day, hour, minute, second, millisecond, weekday (Monday = 1, Sunday = 7) and day of year (1…365/366). Compare the different field order and Sunday code in TwinCAT `TIMESTRUCT` below. |
| TwinCAT `Tc2_Utilities.T_FILETIME` | 8 | Offsets 0…3 hold the low `DWORD`, 4…7 the high `DWORD`. Assemble `high × 2^32 + low` to obtain the 100 ns counter from the 1601 epoch. |
| TwinCAT `Tc2_Utilities.T_FILETIME64` | 8 | `ULINT` containing 100 ns intervals since 1601-01-01, typically UTC. |
| TwinCAT `Tc2_EtherCAT.T_DCTIME64` | 8 | `ULINT` containing nanoseconds since 2000-01-01 00:00:00 UTC in EtherCAT distributed-clock time. |
| TwinCAT `Tc2_Utilities.TIMESTRUCT` | 16 | Eight successive `WORD` fields. Offsets 0, 2, 4, 6, 8, 10, 12, 14 hold year, month, weekday (Sunday = 0), day of month, hour, minute, second and millisecond. |
| TwinCAT `Tc2_System.T_AmsNetID` | 24 | Alias of `STRING(23)` containing six dotted decimal fields. The PLC text allocation includes the terminator. |
| TwinCAT `Tc2_System.ST_AmsAddr` | 8 with the documented member types | Offsets 0…5 hold the six binary `T_AmsNetIdArr` octets in address order. Offsets 6…7 hold the `T_AmsPort` number. |

For **Sunday, 2024-01-07 12:34:56.789**, the two calendar records have the following LE images. The two bytes at offset 4 already have different meanings.

| Record | Bytes at increasing addresses |
|---|---|
| CODESYS `RTS_SYSTIMEDATE` | `E8 07 01 00 07 00 0C 00 22 00 38 00 15 03 07 00 07 00` |
| TwinCAT `TIMESTRUCT` | `E8 07 01 00 00 00 07 00 0C 00 22 00 38 00 15 03` |

**AMS example.** The bytes `01 02 03 04 05 06 53 03` in the LE binary `ST_AmsAddr` represent AMS Net ID `1.2.3.4.5.6` and port 851. In `T_AmsNetID`, the same Net ID begins with ASCII bytes `31 2E 32 2E`. The PLC type selected by the particular API determines which decoder to use.

Sources: [RTS_SYSTIMEDATE field declaration](https://help.plc.abb.com/Libraries/AB290/CODESYS%20-%20System/SysLibs/SysTimeRtc.library_Library/RTS_SYSTIMEDATE.html) · [TwinCAT T_FILETIME](https://infosys.beckhoff.com/content/1033/tcplclib_tc2_utilities/35377675.html) · [CODESYS runtime clocks](https://content.helpme-codesys.com/en/LibDevSummary/date_time.html) · [TwinCAT FILETIME64](https://infosys.beckhoff.com/content/1033/tcplclib_tc2_utilities/10462547339.html) · [TwinCAT DC time](https://infosys.beckhoff.com/content/1033/tcplclib_tc2_ethercat/2267609355.html) · [TwinCAT TIMESTRUCT](https://infosys.beckhoff.com/content/1033/tcplclib_tc2_utilities/35393035.html) · [AMS Net ID text](https://infosys.beckhoff.com/content/1033/tcplclib_tc2_system/31059723.html) · [binary AMS address and member types](https://infosys.beckhoff.com/content/1033/tcplclib_tc2_system/31068939.html).

### Common library aliases

The following rows resolve library names to the actual storage type. Sizes apply to the published declarations on the 32-/64-bit targets in section 1. The declaration in the project's resolved library version determines the ABI.

| Qualified type | Published base | Bytes | Meaning |
|---|---|---:|---|
| `CAA.PVOID`, `CAA.HANDLE` | `__XWORD` | 4 / 8 | Native bit pattern for a general pointer or opaque handle, respectively. Interpret a handle through the API that issued it. TwinCAT's `PVOID` instead resolves to the unsigned native integer `UXINT`. |
| `CAA.COUNT`, `CAA.SIZE`, `CAA.TICK` | `__UXINT` | 4 / 8 | Unsigned native count, size or tick count. The clock API defines the period of a tick. |
| `CAA.IDENT` | `DWORD` | 4 | Identifier bit pattern. |
| `CAA.SIGNED`, `CAA.UNSIGNED` | `INT`, `UINT` | 2 each | Signed and unsigned 16-bit values in the cited library declarations. |
| `CAA.FLOAT` | `LREAL` | 8 with binary64 support | Floating-point value subject to the target qualification in section 3. |
| `CAA.DURATION` | `TIME` | 4 | Unsigned count of milliseconds. |
| `CAA.FILENAME`, `Tc2_System.T_MaxString` | `STRING(255)` | 256 each | Up to 255 payload bytes and one terminator byte. Neither alias changes the 81-byte allocation of an unqualified `STRING`. |

Sources: [CODESYS CAA alias catalog and declarations](https://content.helpme-codesys.com/en/libs/CAA%20Types%20Extern/Current/CAA-Types/fld-CAA-Types.html) · [CODESYS CAA.PVOID](https://content.helpme-codesys.com/en/libs/CAA%20Types%20Extern/Current/CAA-Types/Types/PVOID.html) · [TwinCAT T_MaxString](https://infosys.beckhoff.com/content/1033/tcplclib_tc2_system/31062795.html).

### Optional safety profile and ADS metadata

The **CODESYS Safety Extension** has its own language profile. These widths describe logical values in that product. Use the device and channel declarations to determine the representation in a particular process image or safety communication frame.

| Safety Extension type | Logical width | Meaning |
|---|---:|---|
| `SAFEBOOL` | 1 bit | False or true. The documented safety network-variable mapping packs up to eight such values in one payload byte. |
| `SAFEINT`, `SAFEDINT` | 16 / 32 bits | Signed values, with the numerical ranges of `INT` and `DINT` respectively. |
| `SAFEWORD` | 16 bits | Bit string with positions 0…15. |
| `SAFETIME` | 32 bits | Safety duration. The Safety Extension table prints the value range as 0…2,147,483,647 s. It does not define the raw tick encoding of a particular communication channel. Obtain that encoding from the channel specification. |
| `SAFEBYTE`, `SAFEDWORD` | 8 / 32 bits | Logical I/O channel types under the Extended-level declaration rules in the cited manual. |

The ordinary PLC `BOOL` described in section 2 occupies a byte. The Safety Extension even lists its own `BOOL` as one logical bit. The presence of `SAFE` in a type name does not specify framing, redundancy, or on-wire offsets. TwinSAFE is a separate Beckhoff product with its own device descriptions.

Sources: [CODESYS Safety Extension type table](https://content.helpme-codesys.com/en/CODESYS%20Safety%20Extension/sil3_vardecl_editor_types.html) · [Safety network-variable mapping](https://content.helpme-codesys.com/en/CODESYS%20Safety%20Extension/sil3_snvl_payload.html).

**ADS type identifiers.** A symbol's ADS type ID is metadata. For example, `ADST_BIGTYPE` (65) identifies a structured type, `ADST_STRING` (30) a string and `ADST_WSTRING` (31) a wide string. `ADST_REAL80` (32) is reserved in the cited Beckhoff table. It does not add an 80-bit `REAL` to the TwinCAT PLC language. A raw reader still needs the symbol's declaration, byte size and subitems to decode a structure.

Source: [TwinCAT ADSDATATYPEID enumeration](https://infosys.beckhoff.com/content/1033/tcplclib_tc2_utilities/35330059.html).

## 9. Checking a compiled layout

This read-only layout probe operates on its own local PLC variables. Add the `ST_Sample` DUT from section 6, then add the declaration and implementation below to a test program. The expressions use `SIZEOF` so they can also be used on TwinCAT 4024. On TwinCAT 4026 and CODESYS compiler 3.5.16.0 or later, `XSIZEOF` is the preferred operator. Select the same target, compiler profile, DUTs and packing settings as the application whose bytes will be read.

```iecst
PROGRAM PRG_LayoutProbe
VAR
    stSample     : ST_Sample;
    aRecords     : ARRAY[0..1] OF ST_Sample;
    aMatrix      : ARRAY[1..2,1..3] OF UINT := [10, 20, 30, 40, 50, 60];
    wMarker      : WORD := 16#1234;
    pMarker      : POINTER TO BYTE;
    sText        : STRING(4);
    wsText       : WSTRING(2);
    cbRecord     : __UXINT;
    cbString     : __UXINT;
    cbWideString : __UXINT;
    cbPointer    : __UXINT;
    offCount     : __UXINT;
    strideRecord : __UXINT;
    strideFirst  : __UXINT;
    strideSecond : __UXINT;
    sample21     : UINT;
    sample12     : UINT;
    byFirst      : BYTE;
END_VAR
```

```iecst
cbRecord     := SIZEOF(stSample);
cbString     := SIZEOF(sText);
cbWideString := SIZEOF(wsText);
cbPointer    := SIZEOF(pMarker);

offCount     := ADR(stSample.udiCount) - ADR(stSample);
strideRecord := ADR(aRecords[1]) - ADR(aRecords[0]);
strideFirst  := ADR(aMatrix[2,1]) - ADR(aMatrix[1,1]);
strideSecond := ADR(aMatrix[1,2]) - ADR(aMatrix[1,1]);
sample21     := aMatrix[2,1];
sample12     := aMatrix[1,2];

pMarker      := ADR(wMarker);
byFirst      := pMarker^;
```

| Watch expression | Interpretation |
|---|---|
| `cbString` / `cbWideString` | Expected 5 / 6 bytes. |
| `byFirst` | `16#34` indicates LE and `16#12` indicates BE for the marker. |
| `cbPointer` | 4 or 8 bytes according to the PLC target. |
| `cbRecord` / `offCount` / `strideRecord` | Expected 12 / 4 / 12 for the natural layout described above, or 8 / 1 / 8 with explicit `pack_mode := '1'`. |
| `strideFirst` / `strideSecond` | Actual byte strides for this compiled array declaration. |
| `sample21` / `sample12` | 40 / 20 when the initializer advances the rightmost index first, 20 / 30 when it advances the leftmost first. These observations can be compared with the address strides. |

The address differences above are small, positive offsets within one local object or array. CODESYS documents a `DWORD` result for pointer subtraction even with 64-bit pointers. These expressions are intended for member offsets and strides. [CODESYS pointer arithmetic](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_pointer.html).

For a named `BIT` field, inspect the published bit offset or toggle that field in a local test instance and compare snapshots of the enclosing bytes. Taking `ADR` of an individual `BIT` is invalid. For a string, capture the declared capacity as well as the encoding. For an enum, record its base and numeric map. For a time field, record signedness, epoch and unit. This information, together with aggregate size, byte order, member offsets and array strides, is enough to reproduce the worked decodings without the original IDE.

**Verification boundary.** Byte examples are calculated from the stated representations. The examples were checked independently for integer packing, floating-point bit patterns, string encodings, calendar epochs and record offsets. The ST probe has not been compiled or run for either vendor profile in this review. The byte strides of a particular multidimensional PLC array and the compiled layout of runtime descriptors remain target-specific.

Sources: [CODESYS sizing operators](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_operator_sizeof.html) · [TwinCAT SIZEOF](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2528896907.html) · [TwinCAT XSIZEOF](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/13614027275.html) · [TwinCAT variable metadata](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/3527777675.html).
