# Siemens PLC Data Types — Detailed Reference and Binary Layouts

## 1. Scope, compatibility and size conventions

This reference describes data types for **SIMATIC S7-300, classic S7-1200, S7-1200 G2 and S7-1500** in the **TIA Portal V13–V21** range. It covers elementary values, strings, arrays, structures, user-defined types, pointers, references, block parameters, instruction instances, system types, hardware identifiers, communication structures, technology objects and the types permitted in fail-safe programs.

### 1.1 Compatibility of a declaration

Whether a declaration is valid depends on the CPU family and model, the **configured firmware version**, the TIA Portal release and update, the programming language, the declaration section, the block access mode and any instruction or technology-object version involved. A newer physical CPU firmware does not automatically enable features in a project configured for an older target.

Availability labels have the following meanings:

| Label | Meaning |
|---|---|
| Yes | The family supports the type; the stated version, declaration and application restrictions still apply. |
| No | The type is not available in the specified family or context. |
| Conditional | Support requires the stated firmware, compiler, access mode or declaration context. |
| Verify target | Support or an exact version boundary is not established well enough to give an unconditional answer for that configuration. |

**Classic S7-1200 and S7-1200 G2 are separate targets.** Support must not be transferred automatically between them. S7-1500 variants such as R/H, T/TF, Compact, ET 200SP and Software Controller may also have different instruction, communication and technology-object capabilities.

### 1.2 Meaning of “Size [B]”

**B denotes a byte, equal to 8 bits.** A fixed size describes the representation of a value, identifier, descriptor or the stated system data structure. It does not automatically include the complete data block, block code, call overhead or the data reached through a pointer or identifier.

For a structured object, actual memory allocation can depend on field alignment, padding, the block access mode and the instance layout. A `BOOL` represents one logical bit, while the allocated space depends on context. In particular, a bit count must not be treated as a universal byte offset rule for optimized data.

In string declarations, **`n` is the declared maximum number of characters**, not the current text length. In array-size expressions, **`N` is the total number of elements**. A size marked “not established” is unknown within this reference; it does not mean zero. A zero explicitly given for a special parameter does not mean that the underlying mechanism has no CPU memory overhead.

### 1.3 Type definitions and project-specific names

User-defined structures, named value types and function blocks introduce additional type names. Libraries, instruction versions and technology packages may introduce further system structures and instance types. A project-specific name becomes available when its definition and the required package or instruction version are present.

Two types with the same size are not necessarily interchangeable. Compatibility can depend on the declared name, base type, structure, array bounds, block interface and instruction signature. The type used to identify an object is distinct from the type of the object's complete data or instance state.

### 1.4 Binary-layout scope

A type's **logical value encoding**, its **allocated CPU memory**, and a **communication representation** are separate properties. A 32-bit value always has 32 value bits, but the byte offsets of its containing structure, its physical byte order in an optimized block, and its order in a protocol message do not follow from that width alone.

The descriptions use the following layout categories:

| Layout category | Meaning |
|---|---|
| Logical encoding | Meaning of the value bits: unsigned binary, two's complement, IEEE 754, packed BCD, character code, or a defined set of fields. |
| Standard S7 layout | Relative byte and bit positions for a standard-access S7 value or structure. The starting address comes from the compiled declaration. |
| Published instruction structure | Field offsets belonging to a particular public instruction structure. Its CPU and instruction-version requirements remain part of that definition. |
| Generated or opaque layout | No general raw byte map is specified. Use the public fields, symbolic access or the appropriate instruction interface. |

A hexadecimal example is a sequence of bytes in increasing relative byte-offset order when described as a standard S7 byte sequence. Such an example does not prescribe the wire order of Modbus, OPC UA, a vendor telegram, or another protocol. Bit numbering and byte ordering are distinct: bit 0 has weight 1 within a byte even when the bytes of a multi-byte value are stored most significant first.

The core language types form a finite catalog. User-defined PLC data types, named value types, library FBs and instruction-generated structures add names to that catalog. The size and field layout of each such definition belong to that exact definition and version. A list of predefined names cannot enumerate every type that can exist in an arbitrary TIA project.

## 2. Boolean, bit-string and numeric types

The following catalog covers the 15 Boolean, bit-string, integer and floating-point types. The availability columns describe the CPU family's type catalog for standard programs; they do not imply that every instruction, address area, interface section or historical firmware accepts every listed type.

| Type | Size [B] | Value representation | S7-300 | Classic S7-1200 | S7-1200 G2 | S7-1500 |
|---|---|---|:---:|:---:|:---:|:---:|
| `BOOL` | 1 logical bit; allocation depends on context | Boolean `0` or `1` | Yes | Yes | Yes | Yes |
| `BYTE` | 1 | 8-bit string | Yes | Yes | Yes | Yes |
| `WORD` | 2 | 16-bit string | Yes | Yes | Yes | Yes |
| `DWORD` | 4 | 32-bit string | Yes | Yes | Yes | Yes |
| `LWORD` | 8 | 64-bit string | No | No | Yes | Yes |
| `SINT` | 1 | Signed 8-bit binary integer | No | Yes | Yes | Yes |
| `USINT` | 1 | Unsigned 8-bit binary integer | No | Yes | Yes | Yes |
| `INT` | 2 | Signed 16-bit binary integer | Yes | Yes | Yes | Yes |
| `UINT` | 2 | Unsigned 16-bit binary integer | No | Yes | Yes | Yes |
| `DINT` | 4 | Signed 32-bit binary integer | Yes | Yes | Yes | Yes |
| `UDINT` | 4 | Unsigned 32-bit binary integer | No | Yes | Yes | Yes |
| `LINT` | 8 | Signed 64-bit binary integer | No | No | Yes | Yes |
| `ULINT` | 8 | Unsigned 64-bit binary integer | No | No | Yes | Yes |
| `REAL` | 4 | IEEE 754 binary32 | Yes | Yes | Yes | Yes |
| `LREAL` | 8 | IEEE 754 binary64 | No | Yes | Yes | Yes |

Classic S7-1200 `SINT`, `USINT`, `UINT`, `UDINT` and `LREAL` predate firmware 4.x: they are documented for FW 2.2. The S7-1500 extended numeric types are documented in the V13 era; this does not establish a single minimum 1.x firmware for every extended type and declaration context. S7-1200 G2 supports the long types from its initial **FW 1.0 / TIA V20** generation; it is not a target of TIA V13–V19. The classic S7-1200 and G2 must be treated as separate families.

### 2.0 How to read the byte and bit layouts

In these type entries, **`b0` is the least significant bit of the complete value**, and `b7` is the most significant bit of an 8-bit value. **`B0` means relative byte offset zero of the representation being described**, not an absolute PLC address. Within a byte, bit 0 has weight 1 and bit 7 has weight 128.

Unless a particular table says otherwise, the byte maps below use the **standard S7 representation, with the most significant byte first**. They also define an explicit byte-stream format that an application can deliberately construct. They are not maps of the compiler's private storage in an optimized block. An optimized tag must be accessed symbolically or through an instruction that defines the required serialization; a type's logical bits do not establish its physical optimized offsets. A communication protocol can specify a different byte order.

| Width | Size [B] | Standard S7 byte map, from lowest relative offset to highest |
|---|---:|---|
| 8 bits | 1 | `B0 = b7…b0` |
| 16 bits | 2 | `B0 = b15…b8`; `B1 = b7…b0` |
| 32 bits | 4 | `B0 = b31…b24`; `B1 = b23…b16`; `B2 = b15…b8`; `B3 = b7…b0` |
| 64 bits | 8 | `B0 = b63…b56`; `B1 = b55…b48`; `B2 = b47…b40`; `B3 = b39…b32`; `B4 = b31…b24`; `B5 = b23…b16`; `B6 = b15…b8`; `B7 = b7…b0` |

For example, a standard S7 `WORD` with value `16#1234` has bytes `12 34`; value bit `b0` is byte `B1` bit 0. That is different from a packed sequence of `BOOL` values beginning at byte `B0` bit 0. All byte sequences in this reference use hexadecimal byte notation.

The maps describe the value or the defined structure itself. Alignment before a tag, padding after it, the containing DB, and parameter-passing descriptors are separate allocation questions. A mapped 8-byte format does not establish permission to use an 8-byte absolute operand on a particular CPU.

### 2.1 BOOL

**Logical size: one bit.** `FALSE` is `0`; `TRUE` is `1`. `BOOL` represents a condition, digital signal or logical result, with no sign, exponent or length header.

For a bit-addressed operand such as `DBX a.2`, the value occupies bit 2 of byte `a`, with mask `16#04`. The other seven bits belong to other data or unused space; they are not part of that Boolean value. Consecutive Boolean declarations can be packed where the applicable block layout permits it.

For **S7-1500 optimized blocks**, a Boolean tag occupies **1 B**, including an element of an optimized `ARRAY OF BOOL`. The one-byte allocation does not turn its logical domain into 0–255 or make its other bits application data. Do not transfer this allocation rule to every S7-1200, S7-300, I/O or standard-block context.

### 2.2 BYTE

**Size: 1 B; bits `b7…b0`.** `BYTE` is an 8-bit string with 256 possible patterns, `16#00` through `16#FF`. Every bit is data; there are no reserved bits or header. In the byte map, the complete value is `B0`.

For example, `16#A5` has bits 7, 5, 2 and 0 set. It is useful for masks, raw communication bytes and eight independent flags. Its unsigned numeric interpretation would be 165, but `BYTE` is a bit-string declaration, while `USINT` is an arithmetic declaration. Conversion and expression rules determine how a bit string is used numerically. S7-300 supports `BYTE` without supporting native `USINT`.

### 2.3 WORD

**Size: 2 B; bits `b15…b0`.** `WORD` holds any 16-bit pattern from `16#0000` through `16#FFFF`. In the standard S7 map, `B0` holds bits 15–8 and `B1` holds bits 7–0. All 16 bits are payload.

For example, `16#8001` is `80 01`: value bits 15 and 0 are set. There is no intrinsic sign bit in a `WORD`. The same bits would mean 32,769 as a `UINT`, or −32,767 as an `INT`; those meanings come from the numeric type or operation, not from the `WORD` declaration. Typical applications are control words, status words and masks.

### 2.4 DWORD

**Size: 4 B; bits `b31…b0`.** `DWORD` holds any pattern from `16#00000000` through `16#FFFFFFFF`. Its bytes are `B0 = b31…b24`, `B1 = b23…b16`, `B2 = b15…b8`, and `B3 = b7…b0` in the standard S7 map.

For example, `16#12345678` is `12 34 56 78`. There is no sign, exponent or BCD digit field unless the application deliberately interprets the bits that way. `DWORD`, `DINT`, `UDINT` and `REAL` are all 4 B, but the same pattern has different meanings under those declarations. An arithmetic conversion to `REAL` is not equivalent to reinterpreting the four bytes as an IEEE 754 value.

### 2.5 LWORD

**Size: 8 B; bits `b63…b0`.** `LWORD` holds every 64-bit pattern from `16#0000000000000000` through `16#FFFFFFFFFFFFFFFF`. In the 64-bit standard S7 map, the highest-order byte is `B0`, and the lowest-order byte is `B7`.

For example, `16#0123456789ABCDEF` is `01 23 45 67 89 AB CD EF`. All 64 bits are data; bit 63 is not intrinsically a sign. Use `LINT` or `ULINT` when signed or unsigned arithmetic is intended. `LWORD` is supported by S7-1500 and S7-1200 G2, including G2 FW 1.0 / TIA V20; it is unavailable on S7-300 and classic S7-1200.

### 2.6 SINT

**Size: 1 B; range: −128…127.** `SINT` uses 8-bit **two's complement**, with sign at `b7`. Its complete representation is `B0`. Patterns `16#00…16#7F` represent 0…127; patterns `16#80…16#FF` represent −128…−1.

The general signed rule is important for all four signed integer types. If an unsigned reading of a width-`w` pattern is `U`, its signed value is `U` when the top bit is 0, and **`U − 2^w`** when the top bit is 1. This is not sign-and-magnitude encoding. Thus `SINT` −1 is `FF`, −128 is `80`, and −5 is `FB`.

`SINT` is available on classic S7-1200, S7-1200 G2 and S7-1500, but not S7-300. It is suitable for small signed counts and offsets. A positive value of 128 cannot be stored as the same numeric value in a `SINT`.

### 2.7 USINT

**Size: 1 B; range: 0…255.** Every bit has a positive binary weight: `value = Σ(bi × 2^i)` for `i = 0…7`. There is no sign field, and `B0` contains the whole value. `00` means 0, `80` means 128 and `FF` means 255.

`USINT` is available on classic S7-1200, S7-1200 G2 and S7-1500, but not S7-300. It is appropriate for small nonnegative counts and several fields of `DTL`. Its 1 B width matches `BYTE`, but that does not make the two declarations interchangeable in every instruction.

### 2.8 INT

**Size: 2 B; range: −32,768…32,767.** `INT` is a signed 16-bit two's-complement value. Its sign is `b15`, which is bit 7 of `B0` in the standard S7 byte map. `B0` contains bits 15–8 and `B1` contains bits 7–0.

| Value | Standard S7 bytes |
|---|---|
| 0 | `00 00` |
| 1 | `00 01` |
| 32,767 | `7F FF` |
| −32,768 | `80 00` |
| −1 | `FF FF` |

`INT` is available on all four CPU families. Use it for signed parameters, scaled process values or counts within the range. The upper bit is part of the two's-complement encoding; the remaining bits are not simply the absolute magnitude of a negative value.

### 2.9 UINT

**Size: 2 B; range: 0…65,535.** `UINT` is an unsigned 16-bit binary integer. The high byte is `B0` and the low byte is `B1`, giving **`value = 256 × B0 + B1`** in the standard S7 representation.

`00 00` is 0, `80 00` is 32,768, and `FF FF` is 65,535. There is no sign bit. `UINT` is available on classic S7-1200, S7-1200 G2 and S7-1500, but not S7-300. The `YEAR` member of `DTL` uses `UINT`, with a much smaller calendar-valid range than the integer type's full range.

### 2.10 DINT

**Size: 4 B; range: −2,147,483,648…2,147,483,647.** `DINT` is a signed 32-bit two's-complement value. Its sign is `b31`, located at `B0` bit 7 in the standard S7 representation. The bytes hold bits 31–24, 23–16, 15–8 and 7–0, respectively.

`00 00 00 00` is 0; `7F FF FF FF` is the maximum; `80 00 00 00` is the minimum; `FF FF FF FF` is −1. A pattern with `b31 = 1` has the signed value `U − 2^32`, where `U` is its unsigned reading.

`DINT` is available on all four CPU families. It is useful for larger signed counts, positions and scaled integer calculations. It cannot preserve the positive numeric meaning of every `UDINT` value.

### 2.11 UDINT

**Size: 4 B; range: 0…4,294,967,295.** `UDINT` is an unsigned 32-bit binary integer with no sign field. The standard S7 byte map is `B0 = b31…b24` through `B3 = b7…b0`.

Its numeric value is **`B0 × 2^24 + B1 × 2^16 + B2 × 2^8 + B3`**. `80 00 00 00` therefore means 2,147,483,648, and `FF FF FF FF` means 4,294,967,295.

It is available on classic S7-1200, S7-1200 G2 and S7-1500, but not S7-300. The `NANOSECOND` member of `DTL` uses `UDINT`, but only 0…999,999,999 is valid for that fractional-second field.

### 2.12 LINT

**Size: 8 B; range: −9,223,372,036,854,775,808…9,223,372,036,854,775,807.** `LINT` is a signed 64-bit two's-complement integer. Its sign is `b63`, located at `B0` bit 7 in the standard S7 map; the remaining bytes continue in descending significance through `B7 = b7…b0`.

The minimum has bytes `80 00 00 00 00 00 00 00`, the maximum `7F FF FF FF FF FF FF FF`, and −1 `FF FF FF FF FF FF FF FF`. A high bit of 1 is decoded by subtracting `2^64` from the unsigned reading of the complete pattern.

It is supported by S7-1500 and S7-1200 G2, including G2 FW 1.0 / TIA V20. S7-300 and classic S7-1200 do not provide it. Classic S7-1200 support for the equally wide `LREAL` does not imply support for `LINT`.

### 2.13 ULINT

**Size: 8 B; range: 0…18,446,744,073,709,551,615.** `ULINT` uses all 64 bits as an unsigned binary value. In the standard S7 representation, `B0` has weight `2^56`, `B1` weight `2^48`, continuing down to `B7` with weight 1.

`80 00 00 00 00 00 00 00` means 9,223,372,036,854,775,808, and eight `FF` bytes mean the maximum. It is not a signed representation, and its upper half does not fit numerically into `LINT`.

It is available on S7-1500 and S7-1200 G2, including G2 FW 1.0 / TIA V20, and is unavailable on S7-300 and classic S7-1200. It is useful for large nonnegative totals and counters.

### 2.14 REAL

**Size: 4 B.** `REAL` uses IEEE 754 **binary32** and is available on all four CPU families. Its bit fields are:

| Field | Value bits | Width | Meaning |
|---|---|---:|---|
| Sign `s` | `b31` | 1 bit | 0 positive, 1 negative |
| Stored exponent `e` | `b30…b23` | 8 bits | Exponent with bias 127; endpoint codes have special meanings |
| Stored fraction `f` | `b22…b0` | 23 bits | Fraction below the implicit leading bit |

In the standard S7 byte map, `B0` bit 7 is the sign; `B0` bits 6–0 and `B1` bit 7 form the exponent; `B1` bits 6–0 plus `B2` and `B3` form the fraction. There is no separately stored decimal point.

For a **normal** finite value, `1 ≤ e ≤ 254` and the value is **`(−1)^s × (1 + f/2^23) × 2^(e−127)`**. The implicit leading 1 gives **24 significant binary digits**. Siemens describes the type with a conservative precision of six decimal significant digits; this is not a guarantee that every multi-step calculation preserves six accurate digits.

The positive normal range begins at `2^−126 ≈ 1.17549435 × 10^−38` and ends at `(2 − 2^−23) × 2^127 ≈ 3.40282347 × 10^38`; negative values have the corresponding magnitudes. The encoding also contains these cases:

| Exponent/fraction | Encoded meaning | Example standard S7 bytes |
|---|---|---|
| `e = 0`, `f = 0`, `s = 0` | Positive zero | `00 00 00 00` |
| `e = 0`, `f = 0`, `s = 1` | Negative zero | `80 00 00 00` |
| `e = 0`, `f ≠ 0` | Subnormal: `(−1)^s × (f/2^23) × 2^−126` | Smallest positive encoded subnormal: `00 00 00 01` |
| `e = 255`, `f = 0` | Positive or negative infinity | Positive infinity: `7F 80 00 00` |
| `e = 255`, `f ≠ 0` | NaN, not a number | One NaN pattern: `7F C0 00 00` |

For ordinary values, `REAL#1.0` encodes as `3F 80 00 00` and −2.5 as `C0 20 00 00`. These are value encodings, not a promise that the corresponding raw byte order exists in an optimized DB.

**Arithmetic support is narrower than bit-pattern capacity.** Siemens documents CPU-dependent treatment of subnormals, infinities and NaNs. Most relevant CPUs do not calculate with subnormals as ordinary nonzero operands; older S7-300 variants are an exception, and underflow handling must follow the instruction. A further historical difference is documented for classic S7-1200: V1/V2/V3 equality can report `TRUE` for matching NaN bit patterns, while equality of two NaNs is `FALSE` on V4 and S7-1500. Do not infer CPU arithmetic, exception reporting or comparison semantics solely from an IEEE 754 encoding table.

### 2.15 LREAL

**Size: 8 B.** `LREAL` uses IEEE 754 **binary64**. It is supported by classic S7-1200, S7-1200 G2 and S7-1500, but not by S7-300.

| Field | Value bits | Width | Meaning |
|---|---|---:|---|
| Sign `s` | `b63` | 1 bit | 0 positive, 1 negative |
| Stored exponent `e` | `b62…b52` | 11 bits | Exponent with bias 1023; endpoint codes have special meanings |
| Stored fraction `f` | `b51…b0` | 52 bits | Fraction below the implicit leading bit |

In the standard S7 map, `B0` bit 7 is the sign; `B0` bits 6–0 and `B1` bits 7–4 contain the exponent; `B1` bits 3–0 and `B2…B7` contain the fraction.

For a normal finite value, `1 ≤ e ≤ 2046` and the value is **`(−1)^s × (1 + f/2^52) × 2^(e−1023)`**. The leading 1 is implicit, giving **53 significant binary digits**; Siemens describes a conservative precision of 15 decimal significant digits. The positive normal range is approximately **2.2250738585072014 × 10^−308 through 1.7976931348623157 × 10^308**, with corresponding negative magnitudes.

`e = 0, f = 0` encodes signed zero. `e = 0, f ≠ 0` encodes a subnormal, using exponent −1022 without an implicit leading 1; the smallest positive encoded subnormal is `2^−1074`. `e = 2047, f = 0` encodes infinity, and `e = 2047, f ≠ 0` encodes NaN. As with `REAL`, storage of these patterns does not guarantee that arithmetic, conversion and comparison instructions implement the same behavior on every CPU.

| Value or category | Standard S7 bytes |
|---|---|
| 1.0 | `3F F0 00 00 00 00 00 00` |
| −2.5 | `C0 04 00 00 00 00 00 00` |
| Negative zero | `80 00 00 00 00 00 00 00` |
| Positive infinity | `7F F0 00 00 00 00 00 00` |
| One NaN pattern | `7F F8 00 00 00 00 00 00` |

For classic S7-1200 **FW 2.2**, `LREAL` in a global or instance DB is documented only with **optimized access**, and direct addressing is not supported. The byte map above therefore must not be used as a physical DB map for that configuration. The exact later target/compiler boundary for relaxing this historical DB restriction is not established here.

### 2.16 BCD encodings are not additional native integer types

`BCD16` and `BCD32` occur in Siemens conversion instructions, but they are **encoding names, not extra native variable declaration types**. A 16-bit BCD result is carried in a `WORD`; a 32-bit result is carried in a `DWORD`. Each decimal digit uses four bits with valid digit values 0–9, rather than the binary integer weights of `INT` or `DINT`.

| Encoding | Size [B] | Logical fields | Numeric range | Example standard S7 bytes |
|---|---:|---|---|---|
| Signed `BCD16` | 2 | Top nibble `b15…b12`: sign; `b11…b8`: hundreds; `b7…b4`: tens; `b3…b0`: ones | −999…999 | Canonical +123: `01 23`; canonical −123: `F1 23` |
| Signed `BCD32` | 4 | Top nibble `b31…b28`: sign; seven remaining nibbles: decimal digits from millions to ones | −9,999,999…9,999,999 | Canonical +1,234,567: `01 23 45 67`; canonical −1,234,567: `F1 23 45 67` |

The canonical sign nibble is `0` for positive and `F` for negative. Legacy BCD instructions can determine the sign from the highest bit alone; this does not make arbitrary digit nibbles `A…F` valid decimal digits. TIA V14 changed the displayed interpretation of BCD signs relative to V13 SP1, so monitor notation must not be mistaken for a new storage width or a new native type. `S5TIME` uses its own unsigned BCD/time-base format, described below.

## 3. Time, date and timestamp types

A duration, calendar day, time of day and timestamp have different meanings even when their widths match. The binary duration/date counters below contain one integer count; displayed day/hour/minute components do not occupy separate bytes. `DT` and `DTL` are exceptions with explicit calendar fields. `DT`, `LDT` and `DTL` are grouped here by application even though the type catalog classifies them as complex types.

| Type / equivalent name | Size [B] | Representation | S7-300 | Classic S7-1200 | S7-1200 G2 | S7-1500 |
|---|---:|---|:---:|:---:|:---:|:---:|
| `S5TIME` | 2 | Three BCD duration digits plus a time-base code | Yes | No | No | Yes |
| `TIME` | 4 | Signed two's-complement count of milliseconds | Yes | Yes | Yes | Yes |
| `LTIME` | 8 | Signed two's-complement count of nanoseconds | No | No | Yes | Yes |
| `DATE` | 2 | Unsigned day count from 1990-01-01 | Yes | Yes | Yes | Yes |
| `TIME_OF_DAY` / `TOD` | 4 | Unsigned milliseconds since midnight | Yes | Yes | Yes | Yes |
| `LTIME_OF_DAY` / `LTOD` | 8 | Unsigned nanoseconds since midnight | No | No | Yes | Yes |
| `DATE_AND_TIME` / `DT` | 8 | Packed BCD calendar fields and weekday | Yes | No | No | Yes |
| `DATE_AND_LTIME` / `LDT` | 8 | Nonnegative nanosecond count from 1970-01-01 | No | No | Yes | Yes |
| `DTL` | 12 | Separate binary calendar fields and nanoseconds | No | Yes | Yes | Yes |

The G2 long types are already documented for **FW 1.0 / TIA V20**. The availability of a timestamp type does not imply that every clock, timer, conversion or communication instruction accepts it. None of these representations carries a time-zone identifier or UTC offset. Whether a timestamp is supplied as system time or local time depends on the producing instruction and configuration. Representation in nanoseconds does not imply a clock, timer or measurement with nanosecond accuracy.

All relative byte maps below use the standard S7 / explicitly constructed byte-stream convention, with the most significant byte of a multibyte scalar first. They do not assign physical addresses to optimized variables.

**Reserved name `LDATE`.** `LDATE` is the IEC long date-only type name, but it is not documented as a supported native variable type for the Siemens targets covered by this reference. TIA V21 lists `LDATE` and `TO_LDATE` as reserved identifiers; reservation does not establish support for a variable declaration or conversion. Consequently, no Siemens storage size, value range, epoch or byte/bit layout is assigned to `LDATE` here. It is distinct from the supported 8 B timestamp type `LDT` / `DATE_AND_LTIME`, which includes both date and time.

### 3.1 S5TIME

**Size: 2 B.** `S5TIME` is an unsigned legacy duration format. The low 12 bits encode an integer `N = 0…999` in three BCD digits; bits 12 and 13 select a time base. The duration is **`N × time base`**, with an overall range of **0 through 2 h 46 min 30 s**.

| Value bits | Standard S7 byte location | Width | Meaning |
|---|---|---:|---|
| `b15…b14` | `B0` bits 7–6 | 2 bits | Unused by the legacy timer start interpretation; use zero in a canonical encoding |
| `b13…b12` | `B0` bits 5–4 | 2 bits | Binary time-base code |
| `b11…b8` | `B0` bits 3–0 | 4 bits | Hundreds digit of `N`, BCD 0–9 |
| `b7…b4` | `B1` bits 7–4 | 4 bits | Tens digit of `N`, BCD 0–9 |
| `b3…b0` | `B1` bits 3–0 | 4 bits | Ones digit of `N`, BCD 0–9 |

| Time-base bits `b13 b12` | Multiplier | Nonzero range using that base |
|---|---|---|
| `00` | 10 ms | 10 ms…9 s 990 ms |
| `01` | 100 ms | 100 ms…1 min 39 s 900 ms |
| `10` | 1 s | 1 s…16 min 39 s |
| `11` | 10 s | 10 s…2 h 46 min 30 s |

For example, bytes `21 27` encode base `10` = 1 s and BCD digits 127, hence **127 s**. The canonical maximum is `39 99`. More than one pattern can represent the same duration: `01 00`, `10 10` and `20 01` all encode 1 s using different bases. Comparing raw `WORD` patterns is therefore not a general duration comparison.

The top two unused bits are not a sign and are not extra time-base bits. Legacy timers ignore them when started; that statement must not be extended into a promise that every conversion or validation instruction accepts noncanonical values. A digit nibble above 9 is invalid BCD. The coarser bases cannot preserve every millisecond value in the total range; legacy literal conversion selects a suitable base and can truncate to its resolution.

`S5TIME` is supported by S7-300 and S7-1500, and by neither classic S7-1200 nor S7-1200 G2. It is different from both a `TIME` binary count and a `TIMER` parameter identifying a timer resource.

### 3.2 TIME

**Size: 4 B; unit: 1 ms.** `TIME` has the same integer encoding as a signed 32-bit `DINT`, but its value denotes an interval. It has no epoch and no calendar fields.

Its complete range is **−2,147,483,648…2,147,483,647 ms**, or **−24 d 20 h 31 min 23 s 648 ms through +24 d 20 h 31 min 23 s 647 ms**. Bit `b31` is the sign. Standard S7 bytes `B0…B3` contain bits 31–24, 23–16, 15–8 and 7–0.

`T#1s` has bytes `00 00 03 E8`; `T#-1s` has bytes `FF FF FC 18`. `80 00 00 00` is the minimum and `7F FF FF FF` the maximum. All 32-bit patterns have a signed duration interpretation, although an individual timer instruction can reject negative or otherwise unsuitable parameters. `TIME` is available on all four families.

### 3.3 LTIME

**Size: 8 B; unit: 1 ns.** `LTIME` has the signed 64-bit two's-complement encoding of `LINT`, interpreted as a duration. Its range is **−9,223,372,036,854,775,808…9,223,372,036,854,775,807 ns**.

Expressed in duration components, the endpoints are **−106,751 d 23 h 47 min 16 s 854 ms 775 µs 808 ns** and **+106,751 d 23 h 47 min 16 s 854 ms 775 µs 807 ns**. There are no separately stored day or fractional-second fields.

`B0…B7` follow the 64-bit standard S7 map, from bits 63–56 down to bits 7–0; `b63` is the sign. One second is `00 00 00 00 3B 9A CA 00`, zero is eight `00` bytes, and −1 ns is eight `FF` bytes. It is available on S7-1500 and S7-1200 G2, including G2 FW 1.0 / TIA V20. It is unavailable on S7-300 and classic S7-1200. A nanosecond unit does not establish the execution resolution of a consuming timer.

### 3.4 DATE

**Size: 2 B; unit: one calendar day; epoch: 1990-01-01.** `DATE` is an **unsigned 16-bit binary day count**. In the standard S7 map, `count = 256 × B0 + B1`; neither byte is a BCD year, month or day field.

`00 00` means 1990-01-01, `00 01` means 1990-01-02, and `FF FF` means **2169-06-06**, 65,535 days after the epoch. The last date is the endpoint stated by current TIA V21 type help. There is no negative-date encoding or time-of-day field.

Historical documentation needs a separate qualification: older S7-300/SCL documentation **and the classic S7-1200 FW 2.2 manual** state an endpoint of **2168-12-31**. That endpoint corresponds to day count 65,378 (`FF 62`), so it is a documented calendar-domain restriction rather than the physical limit of 16 bits. The exact compiler/target boundary between these documented endpoints has not been established here. Do not infer acceptance of every date above 2168-12-31 in an old configured target merely because a modern help page lists 2169-06-06. `DATE` itself is available on all four CPU families.

### 3.5 TIME_OF_DAY / TOD

**Size: 4 B; unit: 1 ms.** `TIME_OF_DAY` and `TOD` name the same type: an **unsigned 32-bit count of milliseconds since midnight**. Its valid domain is **0…86,399,999**, representing **00:00:00.000…23:59:59.999**.

The standard S7 byte map runs from `B0 = b31…b24` down to `B3 = b7…b0`. Midnight is `00 00 00 00`, one second after midnight is `00 00 03 E8`, and the final valid millisecond is `05 26 5B FF`. A larger unsigned pattern fits into the storage but is not a valid time of day.

There are no individual hour, minute or second bytes, no date and no elapsed-day counter. A calculation crossing midnight needs a date or explicit rollover rule. `TOD` is available on all four CPU families.

### 3.6 LTIME_OF_DAY / LTOD

**Size: 8 B; unit: 1 ns.** `LTIME_OF_DAY` and `LTOD` name the same **unsigned 64-bit count of nanoseconds since midnight**. Its valid domain is **0…86,399,999,999,999**, representing **00:00:00.000000000…23:59:59.999999999**.

The standard S7 map runs from `B0 = b63…b56` down to `B7 = b7…b0`. Midnight is eight zero bytes; one second after midnight is `00 00 00 00 3B 9A CA 00`; the maximum is `00 00 4E 94 91 4E FF FF`. Larger patterns are not valid merely because they fit within 64 bits.

`LTOD` has no calendar date or multiday interval meaning. It is supported by S7-1500 and S7-1200 G2, including G2 FW 1.0 / TIA V20, and is unavailable on S7-300 and classic S7-1200.

### 3.7 DATE_AND_TIME / DT

**Size: 8 B; resolution: 1 ms.** `DATE_AND_TIME` and `DT` are names for the same packed **BCD** calendar timestamp. Its valid range is **1990-01-01 00:00:00.000…2089-12-31 23:59:59.999**.

For a byte containing two BCD digits, the decimal value is `10 × upper nibble + lower nibble`. That is different from interpreting the byte as an ordinary binary integer. The layout is:

| Relative byte / bits | Size [B] | Contents | Encoding and limits |
|---|---:|---|---|
| `B0` | 1 | Two-digit year | BCD `90…99` = 1990…1999; BCD `00…89` = 2000…2089 |
| `B1` | 1 | Month | Two BCD digits, 01…12 |
| `B2` | 1 | Day | Two BCD digits, 01…31, restricted by month and year |
| `B3` | 1 | Hour | Two BCD digits, 00…23 |
| `B4` | 1 | Minute | Two BCD digits, 00…59 |
| `B5` | 1 | Second | Two BCD digits, 00…59 |
| `B6`, bits 7–4 | 0.5 | Hundreds digit of milliseconds | One BCD digit, 0…9 |
| `B6`, bits 3–0 | 0.5 | Tens digit of milliseconds | One BCD digit, 0…9 |
| `B7`, bits 7–4 | 0.5 | Ones digit of milliseconds | One BCD digit, 0…9 |
| `B7`, bits 3–0 | 0.5 | Weekday | 1 = Sunday, 2 = Monday, …, 7 = Saturday |

Thus **milliseconds = `100 × (B6 >> 4) + 10 × (B6 AND 16#0F) + (B7 >> 4)`**, and weekday is `B7 AND 16#0F`. In this mathematical decoding notation, shifts and masks operate on the indicated unsigned bytes.

For **2024-02-29 12:34:56.123, Thursday**, the bytes are **`24 02 29 12 34 56 12 35`**. The last byte contains millisecond digit 3 in its upper nibble and weekday 5 in its lower nibble. It is not a second complete millisecond byte.

Every BCD digit must be valid and the fields must form a real calendar date. Weekday is a separate field; raw manipulation should keep it consistent with the date and follow the consuming instruction's rules. No bit in this representation encodes a time-zone offset. `DT` is supported by S7-300 and S7-1500, and by neither classic S7-1200 nor G2. Its bytes cannot be reinterpreted as `DTL` or `LDT` to perform a date conversion.

### 3.8 DATE_AND_LTIME / LDT

**Size: 8 B; unit: 1 ns; epoch: 1970-01-01 00:00:00.** `DATE_AND_LTIME` and `LDT` name the same binary timestamp. Its valid nonnegative count is **0…9,223,372,036,854,775,807 ns**, corresponding to **1970-01-01 00:00:00.000000000…2262-04-11 23:47:16.854775807**.

`B0…B7` follow the 64-bit standard S7 map. The epoch is eight zero bytes; one second after the epoch is `00 00 00 00 3B 9A CA 00`; the maximum is `7F FF FF FF FF FF FF FF`. The valid range leaves `b63` zero. A pattern with `b63 = 1` must not be treated as a supported pre-1970 timestamp or an extended unsigned date range.

This is a count with an origin, whereas `LTIME` is a signed interval without an origin. The date and clock components displayed by TIA are obtained from the count; they are not separate stored fields. `LDT` is supported by S7-1500 and S7-1200 G2, including G2 FW 1.0 / TIA V20, and is unavailable on S7-300 and classic S7-1200.

### 3.9 DTL

**Size: 12 B.** `DTL` stores calendar and clock information as separate **binary integer fields**, not BCD and not one 96-bit elapsed-time count. It is supported by classic S7-1200, S7-1200 G2 and S7-1500; it is unavailable on S7-300.

The current documented timestamp domain is **1970-01-01 00:00:00.000000000…2262-04-11 23:47:16.854775807**. This is a validity restriction on the complete timestamp, not the bit capacity of its 12-byte structure. In particular, a year field of 2262 does not make all dates in that year valid.

| Relative offset | Field | Type | Size [B] | Field encoding and valid values |
|---|---|---|---:|---|
| 0–1 | `YEAR` | `UINT` | 2 | Binary year; high byte at 0, low byte at 1 in the standard S7 map; 1970…2262 subject to the full timestamp limit |
| 2 | `MONTH` | `USINT` | 1 | Binary 1…12 |
| 3 | `DAY` | `USINT` | 1 | Binary 1…31, restricted by the selected month and year |
| 4 | `WEEKDAY` | `USINT` | 1 | Binary 1…7; Sunday = 1, Saturday = 7 |
| 5 | `HOUR` | `USINT` | 1 | Binary 0…23 |
| 6 | `MINUTE` | `USINT` | 1 | Binary 0…59 |
| 7 | `SECOND` | `USINT` | 1 | Binary 0…59 |
| 8–11 | `NANOSECOND` | `UDINT` | 4 | Binary fractional second, 0…999,999,999; most significant byte at 8 in the standard S7 map |

The sizes add to **2 + 6 × 1 + 4 = 12 B**. None of these fields is a null terminator, length header or reserved byte. Each member uses ordinary unsigned binary bit weights; its complete integer range is larger than its permitted calendar-field range. The weekday is not entered as an independent component of a `DTL` literal; the value-entry mechanism derives it. Raw edits can create inconsistent fields, so consuming instruction rules remain relevant.

For **2024-02-29 12:34:56.123456789, Thursday**, the standard representation is **`07 E8 02 1D 05 0C 22 38 07 5B CD 15`**. `07 E8` is the binary year 2024, `1D` is binary day 29, and `07 5B CD 15` is the binary integer 123,456,789. This example shows why the bytes of a `DT` cannot be copied into a `DTL` as a conversion.

Several older classic S7-1200 manuals state an upper endpoint in **2554**. Siemens issued a correction to the 03/2014 manual replacing that endpoint with **2262-04-11 23:47:16.854775807**. The current range agrees with that correction. This evidence does not establish a precise firmware or compiler version at which every historical configuration changed behavior; no such boundary is asserted here. Neither the old date nor a year that fits into `UINT` is evidence of a valid modern `DTL` value.

Nanosecond storage does not guarantee nanosecond clock resolution. `DTL` is an ordinary stored value; it does not update itself merely because its type is a timestamp.

## 4. Character and string types

A character value contains one encoded unit. A string contains a capacity header, a current-length header and reserved character storage. **`n` is the declared capacity; `m` is the current length, with `0 ≤ m ≤ n`.** The current length does not change the space reserved for the value.

| Type | Size [B] | Representation | S7-300 | Classic S7-1200 | S7-1200 G2 | S7-1500 |
|---|---|---|:---:|:---:|:---:|:---:|
| `CHAR` | 1 | One 8-bit character code | Yes | Yes | Yes | Yes |
| `WCHAR` | 2 | One 16-bit unit of the supported Unicode subset | No | Conditional | Yes | Yes |
| `STRING` / `STRING[n]` | `n + 2`; default **256** | Two 1 B length fields plus `n` character bytes | Yes | Yes | Yes | Yes |
| `WSTRING` / `WSTRING[n]` | `2 × n + 4`; default **512** | Two 2 B length fields plus `n` two-byte units | No | Conditional | Yes | Yes |

For classic S7-1200, **FW 4.1 / TIA V13 SP1** is a documented supported combination for the wide types. The exact status of a **FW 4.0** target with each later compiler is not established here; do not replace that uncertainty with either universal support or a universal prohibition. Wide types are documented for S7-1500 in the V13 era, but an exact minimum early 1.x firmware for every declaration context is not asserted. G2 support is separate from classic S7-1200 support.

### 4.1 CHAR

**Size: 1 B.** The entire value is an 8-bit character code at `B0`; all eight bits belong to the code. There is no length header, terminator or padding inside the character representation.

The raw pattern space is `16#00…16#FF`. Codes `16#00…16#7F` form the ASCII range; codes above `16#7F` depend on the code page and interface. For example, ASCII `'A'` is byte `41`, a space is `20`, and a line feed is `0A`. An arbitrary value in the 256-pattern space need not be a printable character.

`CHAR` is available on all four CPU families. It does not encode a general Unicode character or a variable-length text. A numerical conversion and a text-code conversion are different operations even when the source and destination each occupy one byte.

### 4.2 WCHAR

**Size: 2 B.** `WCHAR` contains one 16-bit unit in Siemens's supported UTF-16 subset. In the standard S7 representation, `B0` contains bits 15–8 and `B1` bits 7–0; the code-unit value is `256 × B0 + B1`. For example, `'A'` is `00 41`, and the Greek character Ω, U+03A9, is `03 A9`. There is no header.

The **physical pattern space** is `16#0000…16#FFFF`. This must be distinguished from **documented character acceptance**: the current TIA V21 `WCHAR` type page gives **`16#0000…16#D7FF`**. Its `WSTRING` page gives a different, stricter prohibition, excluding **`16#D000…16#FFFF`**. These statements do not agree for `D000…D7FF`; they must not be silently combined into a claim that every raw 16-bit pattern, or every `WCHAR`-documented code, is universally accepted in every string operation.

Full Unicode support, including surrogate pairs for supplementary characters, is not established by the two-byte width. The exact accepted repertoire must follow the actual target/compiler and the text instruction or interface. This documentation discrepancy does not make the two-byte bit layout uncertain.

`WCHAR` is available on S7-1500 and S7-1200 G2, conditionally on classic S7-1200 as described above, and not on S7-300.

### 4.3 STRING / STRING[n]

**Size: `n + 2 B`.** The maximum ordinary `STRING` capacity is **254 one-byte characters**. Omitting `[n]` gives the default capacity **254**, hence **256 B** for the represented value.

| Relative byte offset | Size [B] | Meaning |
|---|---:|---|
| 0 | 1 | Declared maximum length `n`, unsigned binary |
| 1 | 1 | Current length `m`, unsigned binary |
| 2 | 1 | Character 1 |
| 3 | 1 | Character 2 |
| `i + 1`, for `1 ≤ i ≤ n` | 1 per character | Character position `i` |
| 2 through `n + 1` | `n` | Entire reserved character area |

For example, `STRING[5]` containing `'ABC'` has bytes **`05 03 41 42 43 xx xx`**, where `xx` denotes unused storage with no defined text value. The value occupies 7 B even though only three characters are currently used. A `STRING[20]` occupies 22 B whether empty or full.

Characters are left-justified. The length byte determines which positions are text; bytes beyond the current length are undefined and need not be zero. **A C-style null terminator is not part of this format.** A byte `00` inside the used character area is a stored character code, not the rule for ending a Siemens string. An array of `CHAR` without the two length bytes is not the same representation as `STRING`.

`STRING` is available on all four CPU families. Its character interpretation follows the same code-page limitations as `CHAR`; it is not implicitly UTF-8. In standard non-optimized blocks a `STRING` begins on a word boundary, while optimized blocks can place it on any byte boundary. The `n + 2` formula describes the value itself; alignment within a containing object is additional.

For an S7-300 temporary `STRING`, the maximum-length header must be initialized before the string is used, and the current-length/content state must be valid. Do not assume that temporary memory has the initialized headers of a DB variable. Formal-parameter lengths follow their interface-specific rules.

### 4.4 WSTRING / WSTRING[n]

**Size: `2 × n + 4 B`.** `WSTRING` reserves `n` two-byte character units plus a four-byte header. The current TIA V21 declaration limit is **16,382 units**, producing **32,768 B** at that capacity. The default capacity without `[n]` is **254**, producing **512 B**. A 16-bit length field's mathematical capacity is not the compiler's declaration limit.

| Relative byte offset | Size [B] | Meaning in the standard S7 representation |
|---|---:|---|
| 0–1 | 2 | Declared maximum length `n`: unsigned binary, high byte then low byte |
| 2–3 | 2 | Current length `m`: unsigned binary, high byte then low byte |
| 4–5 | 2 | Character unit 1, high byte then low byte |
| 6–7 | 2 | Character unit 2, high byte then low byte |
| `4 + 2 × (i − 1)` through `5 + 2 × (i − 1)` | 2 per unit | Character position `i`, for `1 ≤ i ≤ n` |
| 4 through `2 × n + 3` | `2 × n` | Entire reserved character area |

The two header values count **character units, not bytes**. For example, `WSTRING[3]` containing `'AΩ'` has standard bytes **`00 03 00 02 00 41 03 A9 xx xx`**, a total of 10 B. A `WSTRING[20]` occupies 44 B. The unused positions do not define additional text and need not contain zeros. No null terminator is required by the Siemens format.

The character units have the 16-bit layout described under `WCHAR`; their acceptance is subject to Siemens's Unicode restrictions. In particular, the current `WSTRING` help explicitly prohibits `D000…FFFF`, while the separate `WCHAR` page lists `0000…D7FF`. This is a documentation mismatch about accepted values, not evidence for a different header or a variable-width character area. The format does not establish support for the entire Unicode repertoire.

`WSTRING` is supported by S7-1500 and S7-1200 G2, conditionally by classic S7-1200, and not by S7-300. Some classic S7-1200 system-manual editions describe a 65,534-unit maximum and default. That statement conflicts with the STEP 7 declaration help and must not be treated as a portable legal declaration or default for TIA V13–V21. The 16,382 maximum and 254 default are also documented for S7-1500 in the 09/2014 programming guideline; a classic S7-1200 FW 4.6 label alone does not validate `WSTRING[65534]`.

The current documented limit for an **FC return value or an SCL expression is 1,022 units**, even though a stored variable can be larger. CPU DB/memory limits and interface rules can impose further constraints. Conversion from `WSTRING` to one-byte `STRING` must also respect the destination code page; equal-looking text does not imply an unchanged byte sequence.

### 4.5 String parameters and historical behavior

The `n + 2 B` and `2 × n + 4 B` size formulas describe a stored string value, not the size of a parameter descriptor or the amount copied by every call. A formal string parameter can have rules different from an ordinary local or DB declaration.

| Target / context | Documented rule |
|---|---|
| S7-300 `FB.InOut` and `FC.Input`, `FC.Output`, `FC.InOut` | The formal `STRING` has the standard length 254. |
| Classic S7-1200 and S7-1500, `FB.InOut` and `FC.Input`, `FC.Output`, `FC.InOut` | Do not specify a formal `STRING`/`WSTRING` length in these sections. For `WSTRING`, these sections require optimized block access. |
| `WSTRING` FC return and SCL expression | Maximum 1,022 units in the current declaration help. |
| S7-1200 G2 formal interfaces | The type is supported, but the exact interface section/access-mode rules must be checked for the configured G2 target; the classic S7-1200/S7-1500 interface table alone is not a complete G2 compatibility matrix. |

For S7-1200/S7-1500, **TIA V14** introduced explicit runtime checks for truncation when `(W)STRING` block parameters are transferred in SCL: insufficient destination capacity can truncate the value and set `ENO` to `FALSE`. The same release changed forwarding of `STRING` parameters between optimized and standard blocks to carry length information and report relevant access errors through `ENO`. Therefore identical string layouts do not establish identical parameter-transfer behavior in V13 and V14 or later.

Also from V14, explicit length declarations on local or global `(W)STRING` constants are no longer permitted and are removed during project upgrade. A constant declaration, an ordinary stored string and a formal string parameter must not be assigned one universal capacity rule.

TIA V15.1 also corrects invalid Unicode characters when an older project is upgraded, including replacement by IEC escape sequences in string constants. This is evidence that editor acceptance and upgrade handling can change; it does not resolve the current `WCHAR`/`WSTRING` code-range discrepancy into a single historical firmware rule.

## 5. Arrays, structures and user-defined types

### 5.1 Type and construct catalog

| Type or construct | Size [B] | S7-300 | Classic S7-1200 | S7-1200 G2 | S7-1500 | Meaning and principal conditions |
|---|---|---|---|---|---|---|
| `ARRAY[lo..hi, …] OF T` | Depends on element count, element type, alignment and access mode | Yes | Yes | Yes | Yes | Fixed-size collection of elements of one type; at most six dimensions. The element type must be valid in the declaration location. |
| `ARRAY[*] OF T` | Size of the actual array; parameter-passing overhead unspecified | No | Conditional: TIA V14, FW ≥ 4.2, optimized access | Conditional: optimized access and valid formal-parameter location | Conditional: TIA V14, FW ≥ 2.0, optimized access | Formal array parameter whose bounds are supplied by the actual argument. It does not allocate an array of variable size. |
| `STRUCT … END_STRUCT` | Depends on member types, alignment and access mode | Yes | Yes | Yes | Yes | Anonymous structure with named members, which may have different types. |
| PLC data type / UDT | Depends on its definition, alignment and access mode | Yes | Yes | Yes | Yes | Named reusable structural type; may also define the structure of an entire DB. |
| Named value type / NVT | 1, 2, 4 or 8, matching its base type | No | No | No | Conditional: TIA V19 or later, within software units | Named integer or bit-string type with names assigned to selected values of its base type. |

A complex type is permitted only when both the CPU family and the declaration location allow its members. Support for `STRUCT` or `ARRAY` does not make every parameter type a valid member type.

### 5.2 Fixed arrays — `ARRAY[lo..hi, …] OF T`

An array groups elements of the same type under one name. Each dimension has fixed, inclusive lower and upper bounds. The number of elements in a dimension is `hi − lo + 1`; a multidimensional array contains the product of the element counts of its dimensions. The bounds are part of the declared array type.

| Access mode | Permitted index-value range | Allocation restriction |
|---|---|---|
| Standard access | −32,768…32,767 | Actual capacity is limited by the CPU, DB size, available memory and element size. |
| Optimized access | −2,147,483,648…2,147,483,647 | The index range does not imply that an array containing billions of elements can be allocated. |

Up to six dimensions may be declared within one `ARRAY`. Direct `ARRAY OF ARRAY` declarations are not supported by this construct. Use a multidimensional declaration for several index ranges, or use structures or UDTs containing arrays when a hierarchy is needed.

The storage size depends on the element representation and the applicable layout rules. Bounds describe the type; a fixed array has no user-visible length header comparable to a STRING header. Parameter-passing metadata is a separate matter.

**Element order.** The last subscript changes fastest. For `ARRAY[1..2, 1..3] OF INT`, the order is `[1,1]`, `[1,2]`, `[1,3]`, `[2,1]`, `[2,2]`, `[2,3]`. Where elements have a constant byte stride `S` and no additional row padding, a two-dimensional element has relative offset `((i − lo1) × N2 + (j − lo2)) × S`. Here `N2 = hi2 − lo2 + 1`. For more dimensions, multiply each adjusted index by the product of the lengths of the dimensions to its right.

**Standard storage.** An array starts at an even byte offset and its allocated span ends on a word boundary. A byte-sized element does not acquire a padding byte after every element: padding applies at the appropriate array or nested-structure boundary. An array of structures uses the complete padded structure size as the element stride.

BOOL elements are packed from bit 0 upward. In a one-dimensional standard array, element `k`, counted from zero, is at relative byte `k DIV 8`, bit `k MOD 8`. The array's end is then rounded to an even byte boundary. In a multidimensional standard BOOL array, a new row starts at the next byte; leftover bits in the preceding row are padding. Therefore, flattening the total element count into one uninterrupted bit string can produce wrong offsets.

| Fixed array | Logical contents | Current standard span, beginning at an even offset | Optimized S7-1500 element storage |
|---|---|---|---|
| `ARRAY[0..8] OF BOOL` | 9 Boolean values | 2 B: elements 0–7 at B0.b0–b7; element 8 at B1.b0; B1.b1–b7 unused | 9 B, one byte per element |
| `ARRAY[1..3, 1..3] OF BOOL` | 9 Boolean values | 4 B: each three-bit row starts at B0, B1 or B2; B3 is end padding | 9 B, one byte per element |
| `ARRAY[0..2] OF BYTE` | 3 bytes | 4 B: B0–B2 values; B3 padding | 3 B of elements |
| `ARRAY[1..3] OF INT` | 3 × 16-bit integers | 6 B, stride 2 B | 6 B of elements |
| `ARRAY[1..2, 1..3] OF INT` | 6 × 16-bit integers | 12 B, stride 2 B; second row begins at B6 | 12 B of elements |

The optimized column gives the elements' storage, not the total allocation of the surrounding DB, its reserve, or other members. In optimized S7-1500 blocks, a BOOL consumes one byte, including in an array; its logical domain remains FALSE/TRUE. Classic S7-1200 optimized storage uses packed Boolean data and must not inherit the S7-1500 one-byte rule. A general raw BOOL-array layout for S7-1200 G2 optimized storage is not established here; obtain the actual target's compiled allocation.

**Historical boundary.** S7-1500 arrays of BOOL, BYTE and CHAR had different sizes before TIA V15 depending on whether they were inside a structure. V15 standardized those sizes. The standard examples above express the current documented rules, not a promise that every V13/V14-compiled object has the same span. Check historical offsets before reusing absolute addresses or ANY lengths.

An array element must itself be legal in the destination declaration. Parameter-only types are not made valid DB fields by placing them inside an array. Arrays of references, `ARRAY[…] OF REF_TO T`, are prohibited. Arrays of FB instances use the separate multi-instance mechanism and have additional restrictions.

### 5.3 Open array parameters — `ARRAY[*] OF T`

`ARRAY[*]` declares a formal parameter for an existing array. Its bounds are determined by the actual array passed at the call. It provides a way to operate on arrays of different lengths through one interface while retaining the required element type.

The construct does not provide dynamic allocation, resize an existing array, or allow a global DB field to have an unspecified length. The actual array still has a complete declaration and a definite storage size. No universal byte size or public raw layout is assigned to the formal parameter's passing mechanism.

Open arrays may be multidimensional; for example, `ARRAY[*, *, *] OF INT` is a three-dimensional formal array. The actual argument must have the required element type and number of dimensions. Its bounds can be read with `LOWER_BOUND` and `UPPER_BOUND` for the selected dimension. Every open-array formal parameter must receive an actual argument. Assignment from an open array to a fixed-bound array is not generally permitted.

For ordinary fixed-array assignment or parameter transfer, the documented compatibility checks concern the element type, number of dimensions and number of components. Do not confuse those rules with `REF_TO` array compatibility, which explicitly requires identical bounds as well.

| Declaration context | Permission |
|---|---|
| `FB.InOut` | Conditional: optimized access; on S7-1500, “Set in IDB” must be inactive. |
| `FC.Input` | Conditional: optimized access. |
| `FC.Output` | Conditional: optimized access. |
| `FC.InOut` | Conditional: optimized access. |
| `FB.Input`, `FB.Output`, `FB.Static`, block `Temp`, `FC.Return`, constants and global DB fields | No. |

The first supported thresholds are TIA V14 with classic S7-1200 FW 4.2, and TIA V14 with S7-1500 FW 2.0. S7-1200 G2 supports the construct under its own target and interface rules; the classic S7-1200 and S7-1500 firmware numbers are not G2 requirements. S7-300 does not support `ARRAY[*]`.

### 5.4 Structures — `STRUCT`

`STRUCT` groups named members into a composite value. Members can have different types, including permitted nested structures, UDTs and arrays. It is an anonymous type description rather than a separately named PLC data type.

Structure size depends on the complete member list and the selected access mode. Alignment and padding may separate members, so adding their logical sizes is insufficient to determine every byte offset or the total allocated size. Two structures with equal total size are not necessarily assignment-compatible.

Structure nesting limits depend on the CPU and configured firmware:

| Configuration | Nesting limit or condition |
|---|---|
| Traditional rule, including S7-300, classic S7-1200 and S7-1500 before FW 4.0 | Up to 8 levels; the STRUCT layout rules describe an exception of 9 levels in `InOut`. |
| S7-1500, TIA V20 with FW ≥ 4.0 | Up to 26 hierarchy levels. |
| S7-1200 G2, FW ≥ 4.1 | Up to 26 hierarchy levels; do not apply this extension to earlier G2 firmware. |

The top-level structure counts as a level. Under the 26-level rule, an `ARRAY OF STRUCT/UDT` consumes three hierarchy levels. The current general STRUCT description also limits S7-1200/S7-1500 DBs to 252 anonymous STRUCTs and recommends UDTs when more structural definitions are needed. That statement does not establish an independent G2 limit.

Reference fields are not permitted in a `STRUCT`. Other fields remain subject to their own declaration restrictions. A structure with the same members as an instruction's system type is not automatically interchangeable with that system type.

**Standard-layout example.** A STRUCT declared in the order shown below starts at an even byte offset and occupies 8 B:

| Member, in declaration order | Relative location | Interpretation |
|---|---|---|
| `Ready : BOOL` | B0.b0 | One logical bit |
| `Fault : BOOL` | B0.b1 | One logical bit; B0.b2–b7 remain unused |
| `Code : BYTE` | B1 | One byte |
| `Count : DINT` | B2–B5 | Four bytes; standard word alignment permits a DINT at offset 2 |
| `Tail : BYTE` | B6 | One byte |
| End padding | B7 | Completes the structure's even byte span |

This is a standard S7 layout, not an optimized layout. Standard storage preserves declaration order; optimized storage can reorder and align members according to the target. Neither unused bits nor padding bytes are application fields or guaranteed zero values.

### 5.5 PLC data types — UDTs

A UDT gives a reusable name to a structural definition. The same definition can be used for variables, permitted interface parameters, array elements, nested members and UDT-based DBs. Its size is determined by the members and the applicable data layout, not by the name of the UDT.

A UDT introduces no user-visible per-value type-name field or length header. Its standard representation is the representation of its members, including required padding. In an array of UDTs, the element stride includes each UDT's final padding.

The name identifies a type. Matching byte counts or superficially matching member lists do not establish universal compatibility between differently named UDTs. Assignment and parameter compatibility depend on the compiler's rules for the exact types and context.

The structure nesting limits also apply to UDT hierarchies. The permission to declare a UDT does not automatically permit the same UDT as an I/Q/M PLC tag. In the classic S7-1200, using UDTs as I/Q tags becomes available with FW 4.2; UDTs themselves are available on earlier targets.

An entire UDT-based DB can be passed through `VARIANT` under the applicable interface rules. An entire DB used as the target of a typed reference must be based on a UDT or SDT and satisfy the reference's optimized-access and type restrictions.

### 5.6 Named value types — NVTs

An NVT associates symbolic names with selected values of an integer or bit-string base type. It retains the base type's representation size. Values of the base type that have no declared symbolic name remain permitted, so an NVT does not enforce a closed list of alternatives.

| Size [B] | Permitted NVT base types |
|---|---|
| 1 | `SINT`, `USINT`, `BYTE` |
| 2 | `INT`, `UINT`, `WORD` |
| 4 | `DINT`, `UDINT`, `DWORD` |
| 8 | `LINT`, `ULINT`, `LWORD` |

`BOOL`, `REAL`, `LREAL`, `CHAR`, `WCHAR`, `STRING`, `WSTRING`, date types and time types are not NVT base types. The supported base-type set is limited to the twelve names in the table.

**Binary representation.** The stored value uses the chosen base type's width and encoding. Signed bases use the corresponding two's-complement integer representation; unsigned and bit-string bases use the corresponding unsigned bit pattern. Names do not add stored bytes, and the number of declared names does not choose or reduce the width. The encoding is the assigned numeric value, not the position of its name in the declaration and not automatically a one-hot bit mask. For example, a name assigned −1 with base INT has the value pattern `16#FFFF`, while a name assigned 5 with base BYTE has `16#05`. Byte order and placement follow the containing data's layout rules.

NVTs are available from TIA V19 for S7-1500 software units. They remain unavailable on S7-300, the classic S7-1200 and S7-1200 G2. Supported languages are LAD, FBD, SCL and GRAPH; STL support must not be assumed. An NVT is a type definition, not a constant declared in a block's `Constant` section.

Software units require at least S7-1500 FW 2.6. That infrastructure requirement is insufficient to guarantee NVT support on every FW 2.6 target. A separate universal minimum firmware for NVTs is not specified here; the configured CPU, firmware and TIA version must support the NVT within the software unit. TIA V19 must not be converted into an unconditional FW 3.1 minimum. From TIA V20, NVTs can themselves be library types; their V19 availability does not imply that this library capability already existed in V19.

## 6. Pointers, references and parameter types

### 6.1 Type catalog

| Type or mechanism | Size [B] | S7-300 | Classic S7-1200 | S7-1200 G2 | S7-1500 | Meaning |
|---|---|---|---|---|---|---|
| `POINTER` | 6 — address descriptor | Yes | No in the current catalog; verify historical targets | Verify target | Yes | Traditional S7 address descriptor. |
| `ANY` | 10 — descriptor; interpretation depends on its form | Yes | No in the current catalog; verify historical targets | Verify target | Yes | Describes a data area together with its element type and element count. |
| `VARIANT` | 0 in the type table; CPU memory overhead is unspecified | No | Conditional: interface, instruction and version restrictions | Yes | Yes | Reference to data with type information. |
| `REF_TO T` | Representation size unspecified | No | No | Conditional: reference and declaration restrictions; first G2 firmware unspecified | Conditional: TIA V15, FW ≥ 2.5, optimized access | Typed reference to an allowed object of type `T`. |
| `TIMER` | 2 — identifier | Yes | No | Verify target | Yes | Identifies a SIMATIC timer. |
| `COUNTER` | 2 — identifier | Yes | No | Verify target | Yes | Identifies a SIMATIC counter. |
| `BLOCK_FC` | 2 — identifier | Yes | No | Verify target | Yes | Identifies an FC function. |
| `BLOCK_FB` | 2 — identifier | Yes | No | Verify target | Yes | Identifies an FB code block. |
| `BLOCK_DB` | 2 — identifier | Yes | No | No | No | Classic DB identifier. |
| `BLOCK_SDB` | 2 — identifier | Yes | No | No | No | Classic system DB identifier. |
| `VOID` | Not applicable — no value | Yes | Yes | Yes | Yes | Specifies that an FC returns no value. |
| `PARAMETER` | Unspecified — special system operand | No | Conditional: system-instruction signature | Conditional: system-instruction signature | Conditional: system-instruction signature | Special operand category used for a formal block parameter. |
| `INSTANCE` / instance parameter | Depends on the passing mechanism and FB type | Verify target | Conditional: instance-parameter rules | Conditional: instance-parameter rules | Conditional: instance-parameter rules | Passes an instance of a particular FB; the declaration uses that FB's name. |

Descriptor sizes describe the descriptor itself. Identifier sizes describe the identifier, not the referenced timer, counter, code block, DB or instance data. These special types are not interchangeable with ordinary integers of equal width.

### 6.2 Traditional address descriptor — `POINTER`

`POINTER` is a 6 B address descriptor. Its bytes describe a target; they do not hold the target's data. A four-byte address-register pointer is a related addressing format, not a four-byte declaration of the six-byte parameter type.

#### 6.2.1 Six-byte classic address representation

For the documented traditional S7 form, with bytes in increasing descriptor offset:

| Descriptor byte | Contents |
|---|---|
| B0 | DB number, high byte |
| B1 | DB number, low byte; B0–B1 are zero for a non-DB area |
| B2 | Memory-area code |
| B3 | High byte of the encoded bit address |
| B4 | Middle byte of the encoded bit address |
| B5 | Low byte of the encoded bit address |

The address calculation is `A = 8 × byteOffset + bitOffset`, where `bitOffset` is 0…7. B3–B5 store A most-significant byte first. In the documented classic format, B3.b7–b3 are zero; the 16-bit byte offset occupies B3.b2–b0, all of B4, and B5.b7–b3. The bit offset occupies B5.b2–b0.

For example, `P#DB42.DBX20.3` has the six bytes:

`00 2A 84 00 00 A3`

Here `16#002A = 42` and `16#0000A3 = 20 × 8 + 3`. A block number and a byte offset are separate quantities. Neither the code `16#84` nor the encoded address is a DB number.

The documented classic byte-offset field covers 0…65,535. Actual usable ranges depend on the CPU, area, DB length and instruction. The presence of three address bytes does not grant a portable 21-bit byte-offset range or access beyond a standard DB's address space.

#### 6.2.2 Four-byte address pointers

STL indirect addressing uses a 32-bit pointer in an address register or DWORD operand. It has no separate DB-number word. For ordinary classic area-crossing addressing, its high byte is the area code and its remaining three bytes contain A. Area-internal addressing leaves the area to the instruction.

| Bits of the classic 32-bit value | Meaning |
|---|---|
| b31 | 0 for area-internal addressing; 1 for ordinary area-crossing addressing |
| b30–b27 | Zero in the classic form |
| b26–b24 | Classic area identifier |
| b23–b19 | Zero in the classic form |
| b18–b3 | Byte offset, 0…65,535 |
| b2–b0 | Bit offset, 0…7 |

Examples: `P#20.3` corresponds to `16#000000A3`; `P#M20.3` corresponds to `16#830000A3`. A DB access using this four-byte form must obtain the applicable DB context separately. Adding `P#1.1` to `P#8.7` gives `P#10.0`, because bit positions carry into the next byte.

This classic bit-field description must not be applied blindly to special S7-1500 area encodings. S7-1500 address registers also have their own block-boundary and language-change rules; they do not provide a generally usable address for optimized data.

#### 6.2.3 Area codes and declaration limits

| Hexadecimal code | Area | Scope or distinction |
|---|---|---|
| `16#80` | Peripheral I/O | Traditional S7-300/400 POINTER and ANY area |
| `16#81` | I | Input process image |
| `16#82` | Q | Output process image |
| `16#83` | M | Bit memory |
| `16#84` | DB / DBX | Data block |
| `16#85` | DI / DIX | Instance DB; current ANY documentation restricts this ANY area code to S7-300/400 |
| `16#86` | L | Current local data |
| `16#87` | V | Previous local data |
| `16#01` | Peripheral input | Additional S7-1500 POINTER encoding; not an ANY area code from the classic table |
| `16#02` | Peripheral output | Additional S7-1500 POINTER encoding; not an ANY area code from the classic table |
| `16#C0`, `16#C1`, … | Earlier calling levels' local data | Additional POINTER encodings; not a portable user-defined numbering scheme |

S7-300 and S7-1500 allow POINTER in the applicable interface locations. SCL permits forwarding a POINTER to a called block but does not support ordinary value assignment or general pointer arithmetic with this parameter type. Using `P#` to address data requires standard access; support for a pointer-shaped literal at one system-instruction input does not authorize a user-declared POINTER in every location.

The current classic S7-1200 catalog excludes a general user-declarable POINTER. Older S7-1200 manuals nevertheless document pointer literals and the descriptor, so the historical distinction between instruction operands and general declarations must be preserved. No exact firmware transition for general declaration support is established here.

S7-1200 G2 general POINTER declaration remains **Verify target**. The V21 family overview groups it with S7-1500, while the 02/2025 comparison list explicitly marks POINTER as S7-1500-only in the combined column. An exact G2 target, firmware and declaration must settle that conflict; the page title alone is insufficient.

### 6.3 Data-area and legacy parameter descriptor — `ANY`

ANY uses 10 B. The classic representation has two forms: a data-area form and a parameter form for a block, SIMATIC timer or SIMATIC counter. Decode the type before interpreting the last six bytes.

#### 6.3.1 Data-area form

| Descriptor byte | Contents |
|---|---|
| B0 | `16#10`: S7 syntax identifier |
| B1 | Data-type code |
| B2 | Element count, high byte |
| B3 | Element count, low byte |
| B4 | DB number, high byte |
| B5 | DB number, low byte; zero together with B4 for a non-DB area |
| B6 | Memory-area code |
| B7 | High byte of encoded bit address A |
| B8 | Middle byte of A |
| B9 | Low byte of A |

The address is calculated as `A = 8 × byteOffset + bitOffset`. In the documented classic map, B7.b7–b3 are zero; B7.b2–b0, B8 and B9.b7–b3 contain the 16-bit byte offset, and B9.b2–b0 contain the bit offset. The two-byte count and DB fields are most-significant byte first. In a data-area ANY literal, only BOOL may start at a nonzero bit offset; every other declared element type requires `bitOffset = 0`.

For `P#DB42.DBX20.0 INT 3`, the descriptor is:

`10 05 00 03 00 2A 84 00 00 A0`

The count is three INT elements, so the represented data area contains 6 B. The descriptor itself still occupies 10 B. In general, multiply the count by the size of the code's element type, applying the receiving instruction's restrictions. A BOOL count denotes bits; it is not a byte count. A zero count does not by itself establish a valid non-null operand.

The count field has 16 bits, but this does not guarantee that every count up to 65,535 is accepted by an instruction or fits in the selected memory area. Populate the entire descriptor; leaving fields uninitialized can select a different area or range.

| B1 hexadecimal code | Type |
|---|---|
| `00` | NIL / null value |
| `01` | BOOL; current ANY code table restricts this code to S7-300/400 |
| `02` | BYTE |
| `03` | CHAR |
| `04` | WORD |
| `05` | INT |
| `06` | DWORD |
| `07` | DINT |
| `08` | REAL |
| `09` | DATE |
| `0A` | TOD / TIME_OF_DAY |
| `0B` | TIME |
| `0C` | S5TIME |
| `0E` | DT / DATE_AND_TIME |
| `13` | STRING |

The classic NIL descriptor has B0 = `16#10` and B1–B9 zero. Use the supported NULL syntax in application code instead of assuming that every pointer mechanism has the same null bit pattern.

These are ANY type codes, not instruction opcodes, DB numbers or an enumeration of every modern TIA data type. Newer types such as LINT, LREAL, WCHAR or DTL do not acquire a documented ANY type code merely because the CPU supports them. A structure can be described as a BYTE region, but the ANY then does not preserve its UDT identity or member schema. A STRING argument can also be normalized differently according to language and call context; do not derive its generated ANY count solely from the declared STRING capacity.

#### 6.3.2 Classic parameter form for blocks, timers and counters

This form applies to the supported S7-300/400 parameter use. Current ANY documentation restricts the following codes to those families.

| Type | Code in B1 and B6 |
|---|---|
| `BLOCK_FB` | `16#17` |
| `BLOCK_FC` | `16#18` |
| `BLOCK_DB` | `16#19` |
| `BLOCK_SDB` | `16#1A` |
| `COUNTER` | `16#1C` |
| `TIMER` | `16#1D` |

| Descriptor byte | Contents in this form |
|---|---|
| B0 | `16#10` |
| B1 | Parameter-type code from the table |
| B2–B3 | `00 01`: repetition factor 1 |
| B4–B5 | `00 00` |
| B6 | Parameter-type code, not a data-area code |
| B7 | `00` |
| B8–B9 | Timer, counter or block number, most-significant byte first |

A classic ANY identifying DB42 therefore uses `10 19 00 01 00 00 19 00 00 2A`. An ANY identifying SIMATIC timer T5 uses `10 1D 00 01 00 00 1D 00 00 05`. These identify objects, not the bytes inside DB42 or the elapsed-time field of a timer. In particular, the object number is in B8–B9; B4–B5 are zero. The byte.bit calculation used for the data-area form does not apply.

#### 6.3.3 Access and interface conditions

Traditional ANY addressing does not provide general access to optimized data. S7-1500 ANY targets must use standard access; TIA V14 tightened the syntax check so that previously accepted optimized targets are rejected. In the Temp section on S7-1500, an ANY declaration also requires the declaring block to use standard access.

ANY is prohibited as an SCL FC return type, and SCL assignment is restricted. A descriptor's broad type or area code does not override the receiving instruction's permitted data types, bit alignment, transfer length or memory areas. For example, supported block-copy operations impose extra restrictions on BOOL lengths.

The current classic S7-1200 catalog does not establish general user declarations of ANY, despite historical manuals and instruction signatures documenting the form. General S7-1200 G2 declarations remain **Verify target**, because the V21 grouped catalog conflicts with the earlier comparison list. Descriptor decoding and CPU declaration support are separate questions.

### 6.4 Generic data reference — `VARIANT`

`VARIANT` refers to data and carries information about the referenced type. It allows a common block interface to receive different permitted types, including arrays and UDTs. Access to or manipulation of the referenced data uses the corresponding instructions and their type checks.

The type table gives `VARIANT` a length of **0 B**, while the mechanism still consumes CPU memory. This value neither specifies zero overhead nor gives the size of the referenced data. The internal overhead and byte/bit layout are unspecified. No public field map is provided for an address, type identifier, bounds or a null value. The separate `VREF` SDT has a 12 B representation, but that size and its layout must not be assigned to `VARIANT`.

`VARIANT` is a reference mechanism rather than a container that stores an arbitrary value within itself. It is not permitted in `FB.Static`, global data declarations, constants or an FC result. It can be declared in the allowed input, in-out, temporary and FC output contexts; `FC.Output` and `FB.Output` have different permissions.

The referenced object is also restricted:

- VARIANT is not a general FB-instance parameter. Ordinary FB instances, multi-instances and arrays of multi-instances are excluded; the documented UDT/SDT-derived DB exception is a separate case.
- The general VARIANT description permits a whole DB when it is UDT-based. The V15 compatibility rules also preserve the explicit UDT/SDT-derived DB exception; this does not authorize an arbitrary FB instance DB.
- Passing a permitted element of an ordinary DB is a different case from referencing that whole DB.
- From V15, SCL consistently rejects ordinary instance DBs and write-protected actual arguments, bringing its checking into line with the other languages.

On the classic S7-1200, the `VARIANT` mechanism predates FW 4.1. The expanded operations for identifying, copying, serializing and converting variant data have separate version requirements. Type availability does not make every `VariantGet`, `VariantPut` or `MOVE_BLK_VARIANT` instruction version available.

### 6.5 Typed references — `REF_TO T`

`REF_TO T` declares a reference whose target has a specific type. It supports typed access to the referenced object rather than unrestricted address manipulation. The reference's representation size and raw byte/bit fields are unspecified; no fixed width, native address layout, tag ID field, or all-zero NULL encoding should be assumed for memory calculations or external serialization. A `REF_TO T` declaration also does not allocate a new object of type T.

S7-1500 support starts with TIA V15 and configured FW 2.5, using optimized access. S7-1200 G2 also supports typed references under its own restrictions, but a separate first G2 firmware is unspecified. S7-300 and the classic S7-1200 do not support `REF_TO`.

For structured targets, use a named PLC data type (UDT) or an allowed SDT. An anonymous `REF_TO STRUCT` target is not permitted; this is a different restriction from prohibiting reference fields inside a STRUCT. TIA V15 Update 4 corrected an earlier discrepancy in which the interface rejected that declaration but compilation failed to report it.

Reference declarations are subject to all of the following restrictions:

- `REF_TO BOOL` is prohibited.
- `REF_TO ARRAY[*]` is prohibited.
- Arrays of references are prohibited.
- Reference members in a `STRUCT` are prohibited.
- On S7-1500, references may be declared in `OB.Temp`, `FB.Temp`, `FC.Input`, `FC.Output`, `FC.Temp` and `FC.Return`. They are not permitted in `FB.Input`, `FB.Output`, `FB.InOut`, `FB.Static` or `FC.InOut`.
- A reference is not retentive. Its target may be retentive.
- The initial reference value is `NULL`.

`REF()` creates a reference to permitted data in a global DB with optimized access or in a `Static` section. It cannot be used to reference `Temp` data, PLC tags, block parameters, constants or read-only data. Support for declaring a reference in `Temp` does not permit a temporary variable to become its target.

For array targets, the element type, every bound and the number of dimensions must match the reference type. An equal element count alone does not establish compatibility. When referencing an entire DB, the DB must be based on a UDT or SDT.

The explicitly listed base timer targets are `IEC_TIMER` and `IEC_LTIMER`. The counter list contains `IEC_SCOUNTER`, `IEC_USCOUNTER`, `IEC_COUNTER`, `IEC_UCOUNTER`, `IEC_DCOUNTER` and `IEC_UDCOUNTER`. This list does not establish references to the long counter variants. Permission for a base SDT does not extend to a derived instruction instance type such as `TON` or `CTU`.

A declared reference can point to permitted bit strings, integers, floating-point types, character strings, UDTs, SDTs and arrays of permitted targets. A character-string reference does not contain an explicit STRING/WSTRING length declaration. Use the reference-target list for the configured target rather than assuming that every type in the CPU's general type catalog is referenceable.

For an assignment attempt from VARIANT to a reference to an ARRAY, the bounds must match exactly. The documented S7-1500 compatibility condition also calls for the blocks that create that VARIANT value to be compiled once for FW 2.5. Do not turn this array-specific condition into a fixed raw descriptor format.

References to technology objects have additional rules. On S7-1500 they require FW 3.0 or later, associated with TIA V18. A block can have up to ten technology-object references. `REF_TO ARRAY OF <technology object>` is prohibited, and technology-object references are not created with `REF()`. The S7-1500 FW 3.0 threshold must not be transferred to G2 as a G2 version requirement.

### 6.6 Legacy timer, counter and block parameters

`TIMER`, `COUNTER`, `BLOCK_FC`, `BLOCK_FB`, `BLOCK_DB` and `BLOCK_SDB` each use a 16-bit, 2 B identifier. Their meanings remain distinct:

| Type | Identified object | Restrictions and distinctions |
|---|---|---|
| `TIMER` | SIMATIC timer | Not a duration value and not IEC timer instance memory. `TIME` and `S5TIME` represent time values. |
| `COUNTER` | SIMATIC counter | Not a counter's numeric current value and not an IEC counter state structure. |
| `BLOCK_FC` | FC | Identifies an FC code block; does not represent the block's code or a return value. |
| `BLOCK_FB` | FB | Identifies the FB code block, without creating or passing the state of a particular instance. |
| `BLOCK_DB` | Classic DB | Available as a classic S7-300 parameter type; not interchangeable with `DB_ANY`. |
| `BLOCK_SDB` | Classic system DB | Available as a classic S7-300 parameter type; not a general user structure. |

For the classic stored identifier, B0 contains number bits b15–b8 and B1 contains b7–b0. Thus FC101 has the identifier bytes `00 65`, and T5 has `00 05`. The 16-bit encoding range is not the CPU's guaranteed object-number range; actual timer, counter and block limits are CPU-specific. There are no inline timer state bits or executable block contents in this word.

On S7-300, these identifiers are allowed in `FB.Input` and `FC.Input`, with the other declaration locations excluded in the interface matrix. On S7-1500, `TIMER`, `COUNTER`, `BLOCK_FC` and `BLOCK_FB` are input parameter types. The special `INSTANCE` mechanism has separate `InOut` and `Temp` cases; those cases do not authorize these identifier types in those locations.

Classic S7-1200 does not provide these legacy parameter types. For G2, support for `TIMER`, `COUNTER`, `BLOCK_FC` and `BLOCK_FB` remains **Verify target**. The absence of a legacy instruction family alone does not settle every parameter-declaration case. `BLOCK_DB` and `BLOCK_SDB` are not provided for G2 or S7-1500.

### 6.7 `VOID`, `PARAMETER` and `INSTANCE`

**`VOID`** specifies that a function has no return value. It has no stored value and no byte size. It is a permitted FC result declaration, not a type for PLC tags, DB fields, static variables or other data declarations.

**`PARAMETER`** is a special operand category used in system signatures. For example, the `VARIABLE` input of `GetSymbolName` accepts a formal parameter of the calling block's interface. At that operand, an arbitrary PLC tag or DB field is not an accepted substitute. General-purpose declarations such as a global DB field of type `PARAMETER` are not established; the instruction's operand rules define its use. Its representation size is unspecified.

**`INSTANCE`** denotes the mechanism for passing an instance of a particular FB. The actual declaration type is the name of that FB. It is not an ordinary scalar type, a generic UDT, or a synonym for `BLOCK_FB`. Its passing representation depends on the mechanism and FB type and must not be equated with the size of the instance's stored data.

## 7. Block instances and declaration contexts

### 7.1 Instances and multi-instances

An FB instance holds the state associated with a particular use of an FB. The type in the declaration is the name of an available FB or a supported instruction instance type. Its layout and size depend on that FB, its version, the target and the instance layout.

A multi-instance places an FB instance within another FB's instance data. It therefore represents state, while `BLOCK_FB` only identifies an FB code block. A UDT containing ordinary fields does not automatically replace an FB instance.

| Construct | Size [B] | S7-300 | Classic S7-1200 | S7-1200 G2 | S7-1500 | Principal conditions |
|---|---|---|---|---|---|---|
| FB name as a multi-instance type | Specific FB instance layout | Yes | Yes | Yes | Yes | The FB or instruction instance type must be available and valid in the host FB's instance declaration. |
| `ARRAY[…] OF <FB type>` | Element count and specific instance layout | No | Conditional | Conditional | Conditional | Array of multi-instances in a block with optimized access; target and version restrictions apply. |
| Instance passed as a parameter | Depends on the passing mechanism, not the instance data size | Verify target | Conditional | Conditional | Conditional | Actual and formal instance types must satisfy the instance-parameter mechanism. |

Arrays of multi-instances and instance parameters are available as TIA V14 features for the applicable targets. Their first firmware versions are not automatically identical to the FW 4.2 and FW 2.0 thresholds for `ARRAY[*]`. The exact target must support the particular instance construct.

`DB_ANY` can be used as an actual argument for an instance parameter on classic S7-1200 from FW 4.2 and on S7-1500 from FW 2.0. This excludes instances of system blocks from the Instructions tab, and the argument must not be `NULL`. These conditions apply specifically to the `DB_ANY` argument form. `DB_ANY` identifies a DB and does not impose the structure of a selected FB on an arbitrary DB.

For a supported parameter instance, the declaration type is the called FB's name. `FB.InOut` and `FC.InOut` permit access to the supplied instance and a call of its FB. `FC.Input` provides read access to instance data and `FC.Output` provides write access; these two FC parameter directions do not permit calling the instance. The passed reference does not duplicate the entire persistent instance.

The instance's stored size includes its actual compiler-selected state and interface representation. A multi-instance array uses an instance-specific stride. Summing visible FB parameter widths does not establish the required size, and Temp variables are not part of persistent instance data. An instruction version change may change its instance structure.

S7-300 instance-parameter syntax and support require verification for the exact configuration. They must not be treated as categorically unavailable, and passing an instance DB through `BLOCK_DB` is not an unconditional substitute for the instance-parameter mechanism.

### 7.2 Declaration rules

A type must be permitted both by the CPU's type catalog and by the specific declaration location. An interface-table category containing several complex types does not add an unsupported member of that category to a CPU family. Block access mode, programming language and configured firmware remain additional conditions.

The interface tables describe the current TIA V21 declaration rules. Historical TIA releases and updates can impose different restrictions, so current permission must not be applied automatically to every release from V13 onward. A global DB field and an I/Q/M PLC tag are distinct declaration locations from block interface parameters.

### 7.3 S7-300 interface declarations

In this table, **legacy identifiers** means `TIMER`, `COUNTER`, `BLOCK_FC`, `BLOCK_FB`, `BLOCK_DB` and `BLOCK_SDB`.

| Declaration section | Legacy identifiers | `POINTER` | `ANY` | `VOID` |
|---|---|---|---|---|
| `OB.Temp` | No | No | Yes | No |
| `FB.Input` | Yes | Yes | Yes | No |
| `FB.Output` | No | No | No | No |
| `FB.InOut` | No | Yes | Yes | No |
| `FB.Static` | No | No | No | No |
| `FB.Temp` | No | No | Yes | No |
| `FC.Input` | Yes | Yes | Yes | No |
| `FC.Output` | No | Yes | Yes | No |
| `FC.InOut` | No | Yes | Yes | No |
| `FC.Temp` | No | No | Yes | No |
| `FC.Return` | No | Yes | Conditional: excluded in SCL | Yes |
| `Constant` | No | No | No | No |

Ordinary elementary types and permitted complex types can be used in their normal data sections. `STRING` uses the standard maximum length of 254 in `FB.InOut` and `FC.Input`, `FC.Output` and `FC.InOut`. Constants of type `ARRAY` and `STRUCT` are excluded. SCL does not permit an `ANY` return value.

### 7.4 Classic S7-1200 interface declarations

| Declaration section | `VARIANT` | `ARRAY[*]` | `VOID` |
|---|---|---|---|
| `OB.Temp` | Yes | No | No |
| `FB.Input` | Yes | No | No |
| `FB.Output` | No | No | No |
| `FB.InOut` | Yes | Conditional: FW ≥ 4.2 and optimized access | No |
| `FB.Static` | No | No | No |
| `FB.Temp` | Yes | No | No |
| `FC.Input` | Yes | Conditional: FW ≥ 4.2 and optimized access | No |
| `FC.Output` | Yes | Conditional: FW ≥ 4.2 and optimized access | No |
| `FC.InOut` | Yes | Conditional: FW ≥ 4.2 and optimized access | No |
| `FC.Temp` | Yes | No | No |
| `FC.Return` | No | No | Yes |
| `Constant` | No | No | No |

The firmware threshold for `ARRAY[*]` is associated with TIA V14. The length of a formal `STRING` or `WSTRING` parameter is not specified explicitly in `FB.InOut` or `FC.Input`, `FC.Output` and `FC.InOut`. `WSTRING` in those sections requires optimized block access. A `WSTRING` FC result uses a maximum length of 1022 characters.

### 7.5 S7-1500 interface declarations

Every permitted `REF_TO` declaration requires optimized access and FW 2.5 or later. Every permitted `ARRAY[*]` declaration requires optimized access and FW 2.0 or later. Specialized reference targets can require newer firmware.

| Declaration section | `POINTER` | `ANY` | `VARIANT` | `REF_TO` | `ARRAY[*]` | `DB_ANY` |
|---|---|---|---|---|---|---|
| `OB.Temp` | No | Conditional: standard access | Yes | Yes | No | Yes |
| `FB.Input` | Yes | Yes | Yes | No | No | Yes |
| `FB.Output` | No | No | No | No | No | Yes |
| `FB.InOut` | Yes | Yes | Yes | No | Conditional: “Set in IDB” inactive | Yes |
| `FB.Static` | No | No | No | No | No | Yes |
| `FB.Temp` | No | Conditional: standard access | Yes | Yes | No | Yes |
| `FC.Input` | Yes | Yes | Yes | Yes | Yes | Yes |
| `FC.Output` | Yes | Yes | Yes | Yes | Yes | Yes |
| `FC.InOut` | Yes | Yes | Yes | No | Yes | Yes |
| `FC.Temp` | No | Conditional: standard access | Yes | Yes | No | Yes |
| `FC.Return` | Yes | Conditional: excluded in SCL | No | Yes | No | Yes |
| `Constant` | No | No | No | No | No | No |

`DB_ANY` is a DB identifier with a 2 B base representation. Its specified length in `Temp` is 0 B; this does not describe the size of the identified DB or establish zero internal overhead.

On S7-1500, NVT variables are permitted in the normal OB.Temp, FB.Input/Output/InOut/Static/Temp and FC.Input/Output/InOut/Temp/Return sections within software units, but NVT is not a declaration for the Constant section.

For S7-1500 as well as classic S7-1200, formal STRING/WSTRING lengths are not explicitly specified in `FB.InOut` and `FC.Input`, `FC.Output` or `FC.InOut`. WSTRING requires optimized access in these locations. A WSTRING FC result is limited to 1,022 characters. These are parameter rules, not the storage formula for a concrete string variable.

The `REF_TO` rules are deliberately different for FBs and FCs. An FC may have reference inputs, outputs and a reference result; an FB cannot acquire the same permissions by analogy. `FC.InOut` remains excluded even though `FC.Input` and `FC.Output` are permitted.

For S7-1200 G2, apply its own family catalog, target-specific declaration rules and firmware conditions. The S7-1500 table must not be copied wholesale to G2, particularly for `ANY`, `POINTER` and legacy identifier types whose support remains **Verify target**.

### 7.6 Standard access, optimized access and language restrictions

S7-300 uses standard access and does not provide optimized DBs. In newer CPU families, standard and optimized access have different layout and addressing rules. Byte offsets from a standard-access DB are not transferable to an optimized DB, and UDT layout cannot be calculated solely by adding the member widths.

The declaration of a type in a DB does not automatically authorize its use in the I/Q/M PLC tag table. Structured tags must satisfy the tag table's own type, addressing and length restrictions. Classic S7-1200 FW 4.2 enables UDT-based I/Q tags without changing the earlier availability of UDTs in other contexts.

In SCL, ordinary `POINTER` assignment is excluded, `ANY` assignment is restricted, and `ANY` is not permitted as an FC result. Literal `ARRAY` or `STRUCT` declarations must not be assumed to be valid for `RET_VAL`; named UDT return types require the rules for the exact declaration and target and must not be rejected solely by extending the literal-type restriction.

`AT` declares an alternative view of existing memory in SCL. It is a declaration mechanism, not a data type or a native `UNION`. Its validity depends on the overlaid variable, the overlay size, the declaration section and the access mode. On S7-300, the overlay cannot be wider than the source area. Current textual-interface AT declarations require non-optimized blocks. Historical/compiler-specific exceptions must be checked in the dedicated overlay rules; a permissive declaration in an older project is not a portable optimized layout contract. From V16, BOOL, BYTE, SINT, USINT and CHAR block parameters cannot be overlaid with STRUCT or UDT; an ARRAY overlay must not exceed the overlaid storage.

Retentivity and parameter attributes can also constrain otherwise valid structural types. For example, in TIA V20, a DB with mixed retentivity is not accepted as an actual argument for a formal `STRUCT`. A valid structural shape alone is insufficient to override those parameter rules.


### 7.7 Version changes relevant to layout and parameter compatibility

| TIA or firmware boundary | Consequence |
|---|---|
| TIA V14 | S7-1500 ANY access to optimized data is rejected by the strengthened syntax check. Structured parameter transfer across access modes must account for conversion and copying. |
| TIA V14 SP1 | LAD/FBD use of BLOCK_DB to supply an instruction's instance DB is rejected consistently across CPU families and block types. Use the supported parameter-instance mechanism. The release also corrected multi-instance-array addressing with “Set in IDB”. |
| TIA V15 | S7-1500 BOOL/BYTE/CHAR array sizes are standardized across standalone and STRUCT contexts; historical absolute offsets and ANY lengths need checking. Structured SCL function returns consistently use reference transfer, independently of ENO. |
| TIA V15 | SCL VARIANT actual-argument checks consistently reject ordinary instance DBs and write-protected tags. Nested indexed multi-instance-array addressing was corrected. |
| TIA V15 Update 4 | Compilation consistently rejects an anonymous REF_TO STRUCT target. The update also corrected accesses to REF_TO ARRAY with a variable index. |
| TIA V16 | Instance matching also checks the FB name. Whole-DB MOVE requires identical member counts, types and declaration order, and matching optimized/non-optimized storage; optimized whole-DB operands also have retentivity restrictions. AT overlay checks are stricter. |
| TIA V19 | NVT is introduced for S7-1500 software units. This is distinct from the existing integer/bit-string base representation. |
| TIA V20 | NVT can itself be a library type. Mixed retentive/non-retentive DBs are consistently rejected as actual arguments for a formal STRUCT. S7-1500 FW 4.0 enables 26 hierarchy levels. |
| S7-1200 G2 FW 4.1 in the V21 documentation | The 26-level STRUCT hierarchy extension is documented for this G2 firmware; it is not a classic S7-1200 feature. |

These changes supplement the type-specific introduction thresholds. They do not establish the exact raw span of every historical compiler output. For an existing V13–V21 project that exposes absolute offsets, the saved project/compiler target and its generated layout remain necessary evidence.

## 8. System data types and hardware identifiers

System data types, or SDTs, have a structure defined by the system. Some hold the persistent state of an instruction; others describe an address, connection, diagnostic result, or system object. Their fields and layouts belong to the particular system interface. A user-defined structure with similarly sized fields does not automatically satisfy that interface.

An identifier names an object. Its size describes the identifier value, not the memory occupied by the hardware, block, connection, or technology object that it identifies. Instruction instances additionally contain execution state, so their sizes cannot be reduced to the widths of their visible input and output values.

**Reading sizes and layouts in Sections 8–9.** A published SDT length, a sequence of documented interface bytes, a sum of declared field widths, and the allocation of an optimized DB are different measurements. Numeric offsets below are relative to the start of the identified structure, not to the containing DB. A table explicitly labelled **published interface layout** reproduces the documented field positions for that interface. A table labelled **derived standard layout** applies ordinary standard-access packing to published fields; it is a calculation, not confirmation of an optimized or internal system layout. A **symbolic field contract** describes the fields that may be used without asserting physical offsets. Hidden state and padding do not become user data merely because the overall byte length is known.

For standard-access scalar representations, byte 0 is the most significant byte of a multi-byte value; bit 0 is the least significant bit within its byte or scalar. Optimized memory must be accessed symbolically or through an explicitly defined serialization interface. Protocol layouts and serialized values must not be inferred from a CPU's internal execution representation.

### 8.1 IEC timer state types

| Type | Size [B] | Time value type | Availability and meaning |
| --- | --- | --- | --- |
| `IEC_TIMER` | 16 | `TIME` | Yes: classic S7-1200, S7-1200 G2, and S7-1500. State structure for an IEC timer using 32-bit duration values. It is distinct from the 4-byte `TIME` value carried by its timing parameters. |
| `IEC_LTIMER` | 32 in the general TIA V21 SDT definition; 28 in the S7-1200 G2 manual | `LTIME` | Yes: S7-1200 G2 and S7-1500. No: classic S7-1200. State structure for an IEC timer using 64-bit duration values. The 28-byte and 32-byte values have different published contexts; a single universally applicable allocation is unresolved. Use the layout belonging to the selected CPU and instruction version. |

The specific multi-instance type also expresses the timer operation:

| Type | Size [B] | Operation and value type |
| --- | --- | --- |
| `TP_TIME` | Depends on the instruction instance and version | Pulse timer using `TIME`. The timer produces a pulse of the specified duration when triggered. |
| `TON_TIME` | Depends on the instruction instance and version | On-delay timer using `TIME`. The output becomes active after the input has remained active for the specified delay. |
| `TOF_TIME` | Depends on the instruction instance and version | Off-delay timer using `TIME`. Deactivation of the input starts the delay before the output becomes inactive. |
| `TONR_TIME` | Depends on the instruction instance and version | Retentive on-delay timer using `TIME`. Elapsed timing state is retained according to the instruction's enable and reset behavior. |
| `TP_LTIME` | Depends on the instruction instance and version; the `IEC_LTIMER` base layout has unresolved 28/32-byte contexts | Pulse-timer variant using `LTIME`; requires a CPU and instruction version supporting this variant. |
| `TON_LTIME` | Depends on the instruction instance and version; the `IEC_LTIMER` base layout has unresolved 28/32-byte contexts | On-delay timer variant using `LTIME`; requires a CPU and instruction version supporting this variant. |
| `TOF_LTIME` | Depends on the instruction instance and version; the `IEC_LTIMER` base layout has unresolved 28/32-byte contexts | Off-delay timer variant using `LTIME`; requires a CPU and instruction version supporting this variant. |
| `TONR_LTIME` | Depends on the instruction instance and version; the `IEC_LTIMER` base layout has unresolved 28/32-byte contexts | Retentive on-delay timer variant using `LTIME`; requires a CPU and instruction version supporting this variant. |

The 16-byte `IEC_TIMER` length and the two `IEC_LTIMER` lengths describe the corresponding system state structures. They do not establish one memory size for every derived timer instance, instruction version, or historical CPU firmware.

**Layout status: symbolic field contract; internal state is not a portable binary interface.** The documented timer interface provides `IN` and `Q` as `BOOL`, and `PT` and `ET` as `TIME` or `LTIME`; specific instructions also provide their operation-specific controls, such as reset. Their declared widths do not explain the complete state structure or fix the order of its hidden fields. In particular, do not turn `16` into four guessed `DWORD` members or infer the extra four bytes of the 28/32-byte `IEC_LTIMER` discrepancy as an established padding rule. The general V21 type catalog and the G2 hardware specifications give different lengths; the selected target's generated definition remains decisive.

The internally used timer member `ST` was hidden from TIA V14 onward, and writing it is prohibited. Its visibility in older projects is not evidence that it is a supported public binary field. From TIA V17, external-source import rejects initialization of protected parameters, including protected IEC timer members.

On S7-300, `TP`, `TON`, and `TOF` use the classic system-function-block instance mechanism. Their instance layouts and permitted declaration forms must be taken from the applicable SFB. The modern `IEC_TIMER` length does not define the size of those classic instances.

### 8.2 IEC counter state types

The counter state type determines the representation of the preset and current count values. It also contains instruction state beyond the count value itself.

| Type | Size [B] | Count value type | Availability and meaning |
| --- | --- | --- | --- |
| `IEC_SCOUNTER` | 3 | `SINT` | Yes: classic S7-1200, G2, and S7-1500. Counter state with a signed 8-bit count value. |
| `IEC_USCOUNTER` | 3 | `USINT` | Yes: classic S7-1200, G2, and S7-1500. Counter state with an unsigned 8-bit count value. |
| `IEC_COUNTER` | 6 | `INT` | Yes: classic S7-1200, G2, and S7-1500. Counter state with a signed 16-bit count value. |
| `IEC_UCOUNTER` | 6 | `UINT` | Yes: classic S7-1200, G2, and S7-1500. Counter state with an unsigned 16-bit count value. |
| `IEC_DCOUNTER` | 12 | `DINT` | Yes: classic S7-1200, G2, and S7-1500. Counter state with a signed 32-bit count value. |
| `IEC_UDCOUNTER` | 12 | `UDINT` | Yes: classic S7-1200, G2, and S7-1500. Counter state with an unsigned 32-bit count value. |
| `IEC_LCOUNTER` | 24 | `LINT` | Yes: G2 and S7-1500. No: classic S7-1200. Counter state with a signed 64-bit count value. |
| `IEC_ULCOUNTER` | 24 | `ULINT` | Yes: G2 and S7-1500. No: classic S7-1200. Counter state with an unsigned 64-bit count value. |

In the instruction-specific names, `CTU` identifies an up counter, `CTD` a down counter, and `CTUD` a counter supporting both directions. The suffix identifies the count value type. The suffix does not specify the size of the complete instance.

| Count value type | Up-counter instance | Down-counter instance | Up/down-counter instance | Size [B] |
| --- | --- | --- | --- | --- |
| `SINT` | `CTU_SINT` | `CTD_SINT` | `CTUD_SINT` | Depends on the selected operation, instance, and version |
| `USINT` | `CTU_USINT` | `CTD_USINT` | `CTUD_USINT` | Depends on the selected operation, instance, and version |
| `INT` | `CTU_INT` | `CTD_INT` | `CTUD_INT` | Depends on the selected operation, instance, and version |
| `UINT` | `CTU_UINT` | `CTD_UINT` | `CTUD_UINT` | Depends on the selected operation, instance, and version |
| `DINT` | `CTU_DINT` | `CTD_DINT` | `CTUD_DINT` | Depends on the selected operation, instance, and version |
| `UDINT` | `CTU_UDINT` | `CTD_UDINT` | `CTUD_UDINT` | Depends on the selected operation, instance, and version |
| `LINT` | `CTU_LINT` | `CTD_LINT` | `CTUD_LINT` | Depends on the selected operation, instance, and version; unavailable on classic S7-1200 |
| `ULINT` | `CTU_ULINT` | `CTD_ULINT` | `CTUD_ULINT` | Depends on the selected operation, instance, and version; unavailable on classic S7-1200 |

On S7-300, `CTU`, `CTD`, and `CTUD` use classic SFB instances. Those instances are not assigned the modern state-structure sizes merely because the instruction names or counting operations match.

**Layout status: symbolic field contract; hidden edge and execution state is opaque.** `PV` and `CV` use the count value type selected in the table. Inputs such as `CU`, `CD`, `R`, and `LD`, and outputs such as `Q`, `QU`, and `QD`, belong to the selected counter operation. A published 3/6/12/24-byte state length does not establish the field offsets of all those instruction parameters. These lengths are independently corroborated by the G2 hardware specification. The 64-bit types carry `LINT` and `ULINT` counts; the `UDINT`/`UINT` descriptions in the generic SDT overview are inconsistent with the instruction-specific 64-bit variants and must not be used as their value types.

### 8.3 General system structures

The numeric lengths in this table are the TIA V21 SDT lengths. Earlier versions or instruction-specific structures can have separate requirements. System-defined layouts should be used as supplied, including when the SDT is not displayed in the project's System data types folder.

| Type | Size [B] | Role and target conditions |
| --- | --- | --- |
| `ERROR_STRUCT` | 28 | Runtime-error information used in local error handling. Yes: classic S7-1200, G2, and S7-1500. No: S7-300. |
| `NREF` | 8 | System address information describing an operand. Yes: classic S7-1200, G2, and S7-1500. This is a system reference structure with its own interface role; it does not provide the typed-reference declaration semantics of `REF_TO`. |
| `CREF` | 8 | System address information describing a block. Yes: classic S7-1200, G2, and S7-1500. Its structure and use are defined by the system interface; it is not a general C/C++ pointer. |
| `VREF` | 12 | System variant-reference structure. Yes: classic S7-1200, G2, and S7-1500. Its existence does not permit arbitrary `VARIANT` storage declarations in every data-block location. |
| `SSL_HEADER` | 4 | Header associated with S7-300 system status lists. Yes: S7-300. No: classic S7-1200, G2, and S7-1500 in this SDT catalog. |
| `CONDITIONS` | 52 | Serial receive-condition structure used by the corresponding receive-configuration interface, including `RCV_CFG`. Yes: classic S7-1200. No: S7-300, G2, and S7-1500 in this SDT catalog. |
| `TADDR_Param` | 8 | UDP communication-partner address structure. Yes: classic S7-1200, G2, and S7-1500, subject to the specific communication instruction and CPU. It describes a partner address rather than a complete connection. |
| `TCON_Param` | 64 | Legacy Open User Communication connection-description structure. Yes: classic S7-1200 and G2. Conditional: S7-1500; S7-1500R/H excludes this type. |
| `HSC_Period` | 12 | Period-measurement structure for `CTRL_HSC_EXT` on classic S7-1200. It describes measurement data, whereas `HW_HSC` identifies counter hardware. No: S7-300, G2, and S7-1500 in this SDT catalog. |
| `AssocValues` | 16 | Associated message-value structure used by `Gen_UsrMsg`. Yes: classic S7-1200, G2, and S7-1500, subject to instruction availability. |
| `ResolvedSymbol` | 3 | Handle-like system result for symbolic access at runtime. Conditional: S7-1500 FW 3.0 or later. Verify target: G2. No: S7-300 and classic S7-1200. |
| `TimeTransformationRule` | 96 in the derived standard layout below; optimized allocation not asserted | Public time-zone/changeover structure used by `SET_TIMEZONE`; S7-1500 instruction support starts at FW 1.7. Classic S7-1200 and G2 require their own supported instruction version. |

`ResolvedSymbol` participates in the symbolic runtime-access interface; its 3-byte length does not describe the size of the resolved variable. S7-1500 FW 3.0 supports the specified elementary and hardware types and individual structure or array elements. FW 3.1 extends this mechanism to the supported whole complex types. A `ResolvedSymbol` element in a UDT requires FW 3.0; the TIA V19 tooling context does not change that threshold to FW 3.1. Runtime symbolic access excludes references, `ANY`, `POINTER`, `REMOTE`, technology objects, F-program areas, and certain system data. G2 support requires its own target-specific determination.

#### 8.3.1 Public error-information fields

The following is the **derived standard layout** of the published `ERROR_STRUCT` field contract. Its 28-byte span agrees with the published SDT length. It is suitable for understanding a confirmed standard-access instance; it is not an instruction to parse an arbitrary optimized DB or synthesize system references. The unassigned bytes shown are alignment positions in this calculation, not additional documented members.

| Byte or bit offset | Field | Type | Interpretation |
| --- | --- | --- | --- |
| 0–1 | `ERROR_ID` | `WORD` | Error identifier; 0 means no error. |
| 2 | `FLAGS` | `BYTE` | 0: error outside a block call; 1: error during a block call. |
| 3 | `REACTION` | `BYTE` | 0: ignore a write error; 1: read with substitute zero; 2: skip the instruction. |
| 4 | `CODE_ADDRESS.BLOCK_TYPE` | `BYTE` | 1: OB; 2: FC; 3: FB. |
| 5 | Alignment position | — | No published user field. |
| 6–7 | `CODE_ADDRESS.CB_NUMBER` | `UINT` | Code-block number. |
| 8–11 | `CODE_ADDRESS.OFFSET` | `UDINT` | System/internal code address information. |
| 12 | `MODE` | `BYTE` | Selects the interpretation of operand-address information. |
| 13 | Alignment position | — | No published user field. |
| 14–15 | `OPERAND_NUMBER` | `UINT` | Operand index in the machine operation. |
| 16–17 | `POINTER_NUMBER_LOCATION` | `UINT` | Mode-dependent internal pointer/location information. |
| 18–19 | `SLOT_NUMBER_SCOPE` | `UINT` | Mode-dependent internal slot/scope information. |
| 20 | `DATA_ADDRESS.AREA` | `BYTE` | Operand-area code. |
| 21 | Alignment position | — | No published user field. |
| 22–23 | `DATA_ADDRESS.DB_NUMBER` | `UINT` | Data-block number when meaningful for the access mode. |
| 24–27 | `DATA_ADDRESS.OFFSET` | `UDINT` | Mode-dependent operand-relative address. |

`CREF` is the 8-byte `CODE_ADDRESS` substructure: relative byte 0 `BLOCK_TYPE`, byte 1 alignment, bytes 2–3 `CB_NUMBER`, bytes 4–7 `OFFSET` in this standard layout. `NREF` has the corresponding arrangement for `AREA`, `DB_NUMBER`, and `OFFSET`. The field contract does not make either offset a general-purpose pointer. `MODE` and the error context must be evaluated before interpreting address fields. `VREF` and `ResolvedSymbol` remain system-managed descriptors; no portable byte/bit decomposition is established here.

#### 8.3.2 Status-list header and associated values

`SSL_HEADER` is a 4-byte header used by the classic system-status-list interface. The instruction parameter can be named `SZL_HEADER`; that parameter name is not another type. Its **derived standard layout** is:

| Byte offset | Field | Type | Meaning |
| --- | --- | --- | --- |
| 0–1 | `LENTHDR` | `WORD` | Length in bytes of one returned record. |
| 2–3 | `N_DR` | `WORD` | Number of records transferred, or the reported record count for a header-only query. |

`SSL_HEADER` does not include the list ID and index parameters. Those are separate instruction inputs. The payload size for a normal successful record transfer is `LENTHDR × N_DR`, subject to the selected list and the instruction result.

`AssocValues` has a **published interface layout** of eight `UINT` entries, `Value[1]` through `Value[8]`: entry `i` occupies bytes `2(i−1)` and `2(i−1)+1`. The span is 16 bytes. These entries supply the associated message values numbered 3 through 10 because `Gen_UsrMsg` uses the first two positions for its text identifiers.

#### 8.3.3 Time-zone transformation structure

`TimeTransformationRule` has the following **derived standard layout**. All calendar selectors are numeric binary fields, not BCD.

| Byte offset | Field | Type / Size [B] |
| --- | --- | --- |
| 0–1 | `Bias` | `INT`, 2 |
| 2–3 | `DaylightBias` | `INT`, 2 |
| 4 | `DaylightStartMonth` | `USINT`, 1 |
| 5 | `DaylightStartWeek` | `USINT`, 1 |
| 6 | `DaylightStartWeekday` | `USINT`, 1 |
| 7 | `DaylightStartHour` | `USINT`, 1 |
| 8 | `DaylightStartMinute` | `USINT`, 1 |
| 9 | `StandardStartMonth` | `USINT`, 1 |
| 10 | `StandardStartWeek` | `USINT`, 1 |
| 11 | `StandardStartWeekday` | `USINT`, 1 |
| 12 | `StandardStartHour` | `USINT`, 1 |
| 13 | `StandardStartMinute` | `USINT`, 1 |
| 14–95 | `TimeZoneName` | `STRING[80]`, 82 |

`Bias` is the local-time offset from UTC, in minutes. The general V21 instruction help gives −720…+780, while the classic S7-1200 system-manual interface gives −1439…+1439; apply the selected target's instruction limits. `DaylightBias` is 0…120 minutes, with zero disabling changeover. Month is 1…12, weekday is 1…7 with Sunday=1, and week is 1…5, where 5 denotes the last occurrence of the weekday. The general instruction help marks `TimeZoneName` as ignored. The standard string representation at relative byte 14 contains its maximum-length byte, current-length byte, and 80-byte character capacity.

#### 8.3.4 Serial receive conditions

`CONDITIONS` has public `START` and `END` structures for classic S7-1200 `RCV_CFG`. Its published SDT length is 52 bytes. The following is a **symbolic field contract**; exact internal byte offsets are not specified here:

| Group | Public members and types |
| --- | --- |
| `START` | `STARTCOND : UINT`, `IDLETIME : UINT`, `STARTCHAR : BYTE`, followed by four sequence structures. |
| `START.SEQ[1..4]` | Each sequence has `CTL : BYTE` and `STR`, an array of five `CHAR` elements. |
| `END` | Eight `UINT` members: `ENDCOND`, `MAXLEN`, `N`, `LENGTHSIZE`, `LENGTHM`, `RCVTIME`, `MSGTIME`, `CHARGAP`; then one sequence structure. |
| `END.SEQ` | `CTL : BYTE` and `STR`, an array of five `CHAR` elements. |

The five-character sequences are character arrays, not `STRING[5]`. `CTL` bits 0–4 select characters 1–5 for comparison. `STARTCOND` bits 0–7 select start character, any character, break, idle, and sequences 1–4. `ENDCOND` bits 0–5 select reply timeout, message timeout, character-gap timeout, maximum length, embedded length, and end sequence. Instruction-specific selection rules still apply.

Do not derive the actual sequence stride by adding one control byte to five character bytes: ordinary standard-access array alignment and a system type's internal packing are different questions. The published total alone cannot resolve them.

### 8.4 Hardware identifiers

Hardware identifier types classify the kind of configured object accepted by an instruction. The base-type relationship supplies a representation and a type hierarchy; it does not make unrelated identifiers interchangeable.

| Type | Size [B] | Base type | Meaning |
| --- | --- | --- | --- |
| `HW_ANY` | 2 | `UINT` from TIA V14; `WORD` before V14 | General hardware identifier accepted by interfaces that do not require a narrower hardware category. |
| `HW_DEVICE` | 2 | `HW_ANY` | Identifier of a configured device. |
| `HW_DPMASTER` | 2 | `HW_INTERFACE` | Identifier of a PROFIBUS DP master interface. No: classic S7-1200. |
| `HW_DPSLAVE` | 2 | `HW_DEVICE` | Identifier of a PROFIBUS DP slave device. |
| `HW_IO` | 2 | `HW_ANY` | General identifier for a configured I/O object. |
| `HW_IOSYSTEM` | 2 | `HW_ANY` | Identifier of an I/O system. |
| `HW_SUBMODULE` | 2 | `HW_IO` | Identifier of an I/O submodule. |
| `HW_MODULE` | 2 | `HW_IO` | Identifier of an I/O module. No: classic S7-1200. |
| `HW_INTERFACE` | 2 | `HW_SUBMODULE` | Identifier of a hardware communication interface. |
| `HW_IEPORT` | 2 | `HW_SUBMODULE` | Identifier of an Industrial Ethernet port. |
| `HW_HSC` | 2 | `HW_SUBMODULE` | Identifier of a high-speed-counter hardware function. It identifies hardware rather than holding a count, measurement result, or controller instance. |
| `HW_PWM` | 2 | `HW_SUBMODULE` | Identifier of a pulse-width-modulation hardware function. |
| `HW_PTO` | 2 | `HW_SUBMODULE` | Identifier of a pulse-train-output hardware function. |
| `PORT` | 2 | `HW_SUBMODULE` | Identifier of a communication port for interfaces requiring the `PORT` category. |

These are modern hardware-identifier types for the S7-1200/S7-1500 programming model. S7-300 does not use this modern set. Classic S7-1200 excludes `HW_DPMASTER` and `HW_MODULE`; the presence of the other names still depends on the configured hardware and the instruction accepting the identifier. On G2 and S7-1500, a recognized type name does not imply that every corresponding module or hardware function exists on the selected CPU.

**Scalar byte/bit mapping for every type in this table:** the value is a 16-bit hardware identifier. In a standard-access representation, relative byte 0 contains value bits 15…8 and relative byte 1 contains bits 7…0. Value bit `k` is in byte `1−floor(k/8)`, at bit `k mod 8`. There is no documented universal subfield split into rack, slot, interface class, or channel; use `GEOADDR` and the addressing instructions to obtain those properties. The TIA V14 change from `WORD` to `UINT` changes typing and conversion checks, not the two-byte width.

### 8.5 Event identifiers

| Type | Size [B] | Base type | Meaning and declaration condition |
| --- | --- | --- | --- |
| `AOM_IDENT` | PLC representation size not confirmed | Not confirmed here | System identity name underlying the event-identifier hierarchy. It also occurs in system/API mappings. That occurrence does not establish a general standalone user-declaration form for every TIA release. |
| `EVENT_ANY` | PLC representation size not confirmed | `AOM_IDENT` | General event identifier. |
| `EVENT_ATT` | PLC representation size not confirmed | `EVENT_ANY` | Identifier of an assignable event. |
| `EVENT_HWINT` | PLC representation size not confirmed | `EVENT_ATT` | Identifier of a hardware-interrupt event. |

The event identifier and the organization block responding to the event are different objects. Earlier STEP 7 documentation identifies `AOM_IDENT` as derived from `DWORD`, providing evidence for a historical four-byte representation. That evidence predates this reference's V13–V21 range; current API mappings also use 32-bit values but do not independently prove the complete PLC declaration/allocation contract throughout that range. The table therefore leaves the current physical representation explicitly unresolved. The event class, assignment operation, declaration location, and CPU must all be compatible.

### 8.6 Organization-block identifiers

| Type | Size [B] | Base type | Meaning |
| --- | --- | --- | --- |
| `OB_ANY` | 2 | `INT` | General organization-block identifier. |
| `OB_DELAY` | 2 | `OB_ANY` | Identifier of a time-delay interrupt OB. |
| `OB_TOD` | 2 | `OB_ANY` | Identifier of a time-of-day interrupt OB. |
| `OB_CYCLIC` | 2 | `OB_ANY` | Identifier of a cyclic-interrupt OB. |
| `OB_ATT` | 2 | `OB_ANY` | Identifier of an assignable OB. |
| `OB_PCYCLE` | 2 | `OB_ANY` | Identifier of a program-cycle OB. |
| `OB_HWINT` | 2 | `OB_ATT` | Identifier of a hardware-interrupt OB. |
| `OB_DIAG` | 2 | `OB_ANY` | Identifier of a diagnostic-interrupt OB. |
| `OB_TIMEERROR` | 2 | `OB_ANY` | Identifier of a time-error OB. |
| `OB_STARTUP` | 2 | `OB_ANY` | Identifier of a startup OB. |

These values identify OBs; they do not contain the OB's executable code or its start information. The permitted OB classes and operations depend on the CPU. This modern `OB_*` identifier set does not describe the classic S7-300 parameter system.

Their standard-access layout is the two-byte `INT` layout: byte 0 holds bits 15…8, including the sign bit at bit 7, and byte 1 holds bits 7…0. Valid OB identifiers are constrained by the instruction and CPU; the full signed `INT` range is not a promise that every bit pattern denotes an OB.

### 8.7 Connection, data-block, and other identifiers

| Type | Size [B] | Base type | Meaning and conditions |
| --- | --- | --- | --- |
| `CONN_ANY` | 2 | `WORD` | General communication-connection identifier. |
| `CONN_PRG` | 2 | `CONN_ANY` | Identifier used by the programmed/open UDP communication interfaces that require this type. |
| `CONN_OUC` | 2 | `CONN_ANY` | Identifier of an Open User Communication connection. A connection identifier is distinct from a TCP/UDP port number and from a `TCON_*` connection-description structure. |
| `CONN_R_ID` | 4 | `DWORD` | `R_ID` connection identifier for the corresponding communication interface. No: classic S7-1200. |
| `DB_ANY` | 2 as a base representation; 0 in `Temp` | `UINT` | Data-block identifier. The zero-length `Temp` declaration is a specific allocation rule; it does not mean the identified DB has zero length. |
| `DB_WWW` | 2 as a base representation; 0 in `Temp` | `DB_ANY` | Identifier of a web-page data block. The same explicit zero-length `Temp` rule applies. |
| `DB_DYN` | 2 | `DB_ANY` | Identifier of a dynamic data block. The zero-length `Temp` exception for `DB_ANY` and `DB_WWW` is not established for this type. |
| `REMOTE` | 10 | `ANY` | System remote-address descriptor, including the listed classic S7-1200 system-operand context. Its mapping to `ANY` does not make general `ANY` declarations available on classic S7-1200, and does not authorize arbitrary `REMOTE` fields in DBs. The instruction and declaration context determine admissibility. |
| `RTM` | 2 | `UINT` | Identifier of an operating-hours counter. The identifier does not itself contain the counter's accumulated operating time. |
| `PIP` | 2 | `UINT` | Identifier of a process-image partition. No: classic S7-1200. |

Most of this modern connection/block identifier set is available for the applicable classic S7-1200 interfaces, with `CONN_R_ID` and `PIP` excluded. S7-300 uses its classic parameter types instead. G2 and S7-1500 support still depends on the object and operation selected.

On S7-1500 FW 2.0 or later, `DB_ANY` can be used as an actual argument for an instance parameter under the instance-parameter rules. This excludes instances of system blocks from the Instructions tab, and the argument must not be `NULL`. Identifying a DB does not change that DB's structure into the instance type expected by an arbitrary FB.

For 2-byte entries based on `UINT` or `WORD`, the standard byte/bit mapping is the same as in Section 8.4. For `CONN_R_ID`, byte 0 contains bits 31…24, then bytes 1, 2, and 3 contain bits 23…16, 15…8, and 7…0. These are scalar identifiers, not packed protocol records. `REMOTE` is a system address descriptor; its association with `ANY` does not establish a universally usable raw layout for all remote-address contexts.

TIA V14 also changed conversions when a `DB_ANY` parameter is accessed with `DB_ANY.%DB(B|W|D)`: the normal implicit-conversion rules apply from V14. Identical identifier width across V13–V21 therefore does not imply identical conversion behavior in an upgraded program.

From TIA V17, derived identifiers cannot be used directly in SCL arithmetic or Boolean operations; use a temporary variable or conversion of the appropriate base type. The same release forbids passing a `REMOTE` actual operand stored in an instance, and forbids connecting a `REMOTE` actual operand to a `VARIANT` formal parameter. These are declaration and parameter-passing restrictions, not changes to the identifier's published bit width.

### 8.8 Alarm identifier types

| Type | Size [B] | Target and meaning |
| --- | --- | --- |
| `C_ALARM_S` | 4 | S7-300 alarm-number type based on `DWORD`. Used with `ALARM_S`, `ALARM_SQ`, `ALARM_D`, `ALARM_DQ`, and related alarm operations. The alarm-number declaration belongs in the FB's `Static` section. |
| `C_ALARM`, `C_ALARM_8`, `C_ALARM_8P` | Not confirmed for these types | S7-400 alarm-type family. These names do not extend the S7-300 alarm-type set. |
| `C_NOTIFY`, `C_NOTIFY_8P` | Not confirmed for these types | S7-400 notification-type family. These names do not establish corresponding S7-300 declarations. |
| `C_AR_SEND` | Not confirmed | S7-400 archive-send type; outside the S7-300 type set. |
| `C_ALARM_SD` | 4 (`DWORD` base) | Type restricted to S7-410H; it must not be treated as a general S7-300 or S7-1500 alarm type. |

`C_ALARM_S` uses the standard four-byte `DWORD` bit ordering. Its identifier bits are not a decomposition of the alarm text, acknowledgment state, or associated values. The S7-400/410 entries are boundary information for shared Siemens catalogs, not additions to the requested CPU families.

## 9. Communication, diagnostic and technology types

Specialized types belong to particular instructions, protocols, or technology objects. Their availability depends on CPU model, communication hardware, firmware, and the selected instruction or technology version. An instruction version is independent of the CPU firmware number: `TMAIL_C` V4.0, for example, does not mean CPU FW 4.0.

Where a size is not confirmed, the relevant SDT definition or instance layout determines the allocation. Technology-object data, FB-instance data, object identifiers, and parameters carrying an object reference have different memory meanings.

This section is an instruction-specific inventory of the named communication, diagnostic and technology interfaces described below. It is not a closed list of every type that any installed Siemens option, instruction version, module package, generated server interface, or user library can create. Nested technology-object field types and project-generated names form a separate, versioned catalog. A name in a C# or Web API mapping also does not, by itself, prove a general PLC declaration form.

### 9.1 Open User Communication connection descriptions

| Type | Size [B] | Role and conditions |
| --- | --- | --- |
| `TCON_Param` | 64 | Legacy connection-description SDT for the applicable OUC connection interface. Its fields describe the connection rather than merely naming it. Excluded on S7-1500R/H. |
| `TCON_IP_v4` | 14-byte documented field span; secure wrapper reserves 16 for `ConnPara` | IPv4 TCP/UDP connection description; use the published interface layout below and preserve the selected target's embedding/alignment. |
| `TCON_QDN` | 266-byte documented field span; secure wrapper reserves 272 for `ConnPara` | Domain-name connection description. The documented CPU thresholds are classic S7-1200 FW 4.4 and S7-1500 FW 2.0; CP routing conditions remain separate. |
| `TCON_IP_V4_SEC` | 28-byte documented interface span | Secure IPv4 connection-description variant. Classic S7-1200 FW 4.3 documentation introduces the feature; the V21 connection-configuration description specifies FW 4.4 for its CPU configuration context. CPU, hardware, firmware, and instruction version remain decisive. |
| `TCON_QDN_SEC` | 284-byte documented interface span | Domain-name secure connection description; classic S7-1200 FW 4.4 and S7-1500 FW 2.0 in the documented CPU configuration context. |
| `TCON_IP_RFC` | 80-byte documented interface span | ISO-on-TCP connection description, including two 34-byte transport selectors. |
| `TCON_ISOnative` | Not confirmed; depends on the SDT definition | Native ISO connection-description variant for communication hardware and instructions supporting that connection class. |
| `TCON_PHONE` | Not confirmed; depends on the SDT definition | Phone-connection description for the corresponding supported communication hardware and interface. Its presence in the type family does not imply a phone interface on every CPU. |
| `TCON_FDL` | Not confirmed; depends on the SDT definition | FDL connection-description variant for the corresponding fieldbus communication interface. |
| `TCON_Configured` | 5-byte documented field span; containing allocation not established here | Selects an already configured connection for the applicable `TSEND_C`/`TRCV_C` interface; distinct from a programmed-connection description. |
| `TCON_DNS` | Historical definition; not established here | Earlier domain-name connection description superseded by `TCON_QDN`. Retained as a migration name, not recommended for new connection descriptions. |
| `TCON_TCPTermination` | One public `BOOL`; full allocation target/version dependent | `TCONSettings` option controlling reset versus graceful TCP finish. |
| `TCON_IPMulticastTTL` | One public `USINT`; full allocation target/version dependent | S7-1500 `TCONSettings` option controlling multicast TTL. |
| `TSelector` | 34-byte documented field span | Nested transport selector: `TSelLength : UINT` followed by `TSel : ARRAY[1..32] OF BYTE`. Its use is established inside the relevant connection SDT; general standalone declarations are not assumed. |

Classic S7-1200 FW 4.4 extends DNS use. On S7-1500R/H, the firmware-specific secure-connection rules remain applicable even when a description type has a `_SEC` suffix; the suffix alone does not guarantee active TLS for every release.

`TCONSettings` is an instruction for connection properties. It is not an elementary type or a replacement for a `TCON_*` structure. Its described application requires S7-1500 FW 2.9 or later, or classic S7-1200 FW 4.5 or later.

#### 9.1.1 Legacy connection-description layout

`TCON_Param` has the following **published interface layout**, spanning exactly 64 bytes. Arrays are stored in index order. Length fields describe the meaningful part of a fixed-capacity array; they do not resize that array.

| Bytes | Field | Data type |
| --- | --- | --- |
| 0–1 | `block_length` | `UINT`, fixed value 64 |
| 2–3 | `id` | `CONN_OUC` |
| 4 | `connection_type` | `USINT` |
| 5 | `active_est` | `BOOL` at the documented byte position |
| 6 | `local_device_id` | `USINT` |
| 7 | `local_tsap_id_len` | `USINT` |
| 8 | `rem_subnet_id_len` | `USINT` |
| 9 | `rem_staddr_len` | `USINT` |
| 10 | `rem_tsap_id_len` | `USINT` |
| 11 | `next_staddr_len` | `USINT` |
| 12–27 | `local_tsap_id` | `ARRAY[1..16] OF BYTE` |
| 28–33 | `rem_subnet_id` | `ARRAY[1..6] OF USINT` |
| 34–39 | `rem_staddr` | `ARRAY[1..6] OF USINT` |
| 40–55 | `rem_tsap_id` | `ARRAY[1..16] OF BYTE` |
| 56–61 | `next_staddr` | `ARRAY[1..6] OF BYTE` |
| 62–63 | `spare` | `WORD`, reserved |

Connection type 17 denotes TCP, 18 ISO-on-TCP, and 19 UDP in this legacy structure. In a port-bearing TSAP array the high byte comes first. An IPv4 address occupies the first four `rem_staddr` elements. For the documented ISO-on-TCP TSAP encoding, the second TSAP byte contains slot number in bits 0…4 and rack number in bits 5…7; this encoding belongs to the TSAP field, not to a `HW_*` identifier. Fields that an interface declares unused must retain that interface's prescribed values.

#### 9.1.2 Modern connection-description layouts

The following are **published interface positions**. A field span does not establish the padded allocation of a nested instance. In particular, do not infer the secure variants by appending fields immediately after the last visible member of the non-secure variants.

| Structure | Published byte positions |
| --- | --- |
| `TCON_IP_v4` | 0–1 `InterfaceID : HW_ANY`; 2–3 `ID : CONN_OUC`; 4 `ConnectionType : BYTE`; 5 `ActiveEstablished : BOOL`; 6–9 `RemoteAddress : ARRAY[1..4] OF BYTE`; 10–11 `RemotePort : UINT`; 12–13 `LocalPort : UINT`. |
| `TCON_QDN` | 0–1 interface identifier; 2–3 connection identifier; 4 connection type; 5 active-establishment flag; 6–261 `remote_qdn : STRING[254]`; 262–263 remote port; 264–265 local port. Types of the first six bytes and port fields match the corresponding IPv4 structure. |
| `TCON_IP_RFC` | 0–1 interface identifier; 2–3 connection identifier; 4 connection type; 5 active-establishment flag; 6–7 spare area; 8–11 IPv4 address; 12–45 remote `TSelector`; 46–79 local `TSelector`. Each selector begins with a two-byte `UINT` length, then 32 bytes of selector capacity. |
| `TCON_Configured` | 0–1 `InterfaceID : HW_ANY`; 2–3 `ID : CONN_OUC`; 4 `ConnectionType : BYTE`, value 254. |

The 256-byte domain-name area is one `STRING[254]`, including its two-byte header. It is not an array of 254 independent `STRING` values, despite ambiguous wording in some connection tables.

The secure wrappers publish these positions:

| Field | Type | `TCON_IP_V4_SEC` bytes | `TCON_QDN_SEC` bytes |
| --- | --- | --- | --- |
| `ConnPara` | Corresponding non-secure SDT | 0–15 | 0–271 |
| `ActivateSecureConn` | `BOOL` | 16 | 272 |
| `TLSServerReqClientCert` | `BOOL` | 17 | 273 |
| `ExtTLSCapabilities` | `WORD` | 18–19 | 274–275 |
| `TLSServerCertRef` | `UDINT` | 20–23 | 276–279 |
| `TLSClientCertRef` | `UDINT` | 24–27 | 280–283 |

The two `BOOL` parameters have separately documented byte positions; do not bit-pack them together when reproducing this interface. `ExtTLSCapabilities` bit 0 requests the documented client-side alternate-name validation; bits 1…15 are reserved in this definition. The unaccounted tail of each `ConnPara` area is not assigned invented member names. Check the actual system definition before creating or parsing a raw record for a particular target.

`TCON_TCPTermination` exposes `GracefulShutdown : BOOL` (`FALSE` for reset, `TRUE` for finish). `TCON_IPMulticastTTL` exposes `IPMulticastTTLValue : USINT`, valid 1…255. Their logical field definitions do not publish a universal containing-structure allocation. The TTL feature is documented for S7-1500; a combined S7-1200/G2 help heading does not extend it automatically to those CPUs.

### 9.2 Partner address structures

| Type | Size [B] | Role and conditions |
| --- | --- | --- |
| `TADDR_Param` | 8 | Partner-address structure for the applicable UDP interface. It identifies the communication partner for the operation without providing a complete connection description. |
| `TADDR_SEND_QDN` | 260-byte published interface span | Send-partner domain name and port; corresponding S7-1500 interface from FW 2.0. |
| `TADDR_RCV_IP` | 20-byte published interface span | Receive-partner IPv4/IPv6 address and port; corresponding S7-1500 interface from FW 2.0. |

A port-number field, a `CONN_OUC` connection identifier, a `HW_INTERFACE` hardware identifier, a `TADDR_*` address, and a `TCON_*` connection description have separate roles even when individual fields share a representation such as `WORD`.

**Published interface layouts:**

| Type | Byte positions and fields |
| --- | --- |
| `TADDR_Param` | 0–3 `rem_ip_addr : ARRAY[1..4] OF USINT`; 4–5 `rem_port_nr : UINT`; 6–7 `reserved : WORD`, zero. |
| `TADDR_SEND_QDN` | 0–255 `RemoteQDN : STRING[254]`; 256–257 `RemotePort : UINT`; 258–259 `reserved : WORD`, zero. |
| `TADDR_RCV_IP` | 0–15 `RemoteAddress : ARRAY[1..16] OF BYTE`; 16–17 `RemotePort : UINT`; 18 `RemAddrType : BYTE`; 19 `reserved : BYTE`, zero. |

Ports use the high byte first. IPv4 octets occur in dotted-address order; for example `192.168.2.3` is `C0 A8 02 03`. `RemAddrType=3` indicates IPv4 and `RemAddrType=4` IPv6. The 16-byte address capacity alone does not imply that every CPU/interface supports IPv6 communication.

### 9.3 Network-configuration structures

`T_CONFIG` accepts configuration data through its `CONF_DATA` interface. The data contains the appropriate `IF_CONF_*` records for the requested configuration operation.

| Type or nested structure | Size [B] | Role and conditions |
| --- | --- | --- |
| `IF_CONF_HEADER` | 6 | Published header layout: three `UINT` fields. |
| `IF_CONF_V4` | 18 | Published IPv4 interface record. |
| `IF_CONF_NOS` | 246 capacity | Published station-name record; the record's active `Length` can be shorter. |
| `IF_CONF_NTP` | 22 | Published NTP record. S7-1500 `T_CONFIG` support requires FW 1.8 with the TIA V13 SP1 Update 4 generation. |
| `IF_CONF_V6` | 22 | Published IPv6 record; the documented `T_CONFIG` application requires the relevant CP 154x-1. |
| `IF_CONF_MAC` | 12 | Public MAC-address record. The V21 `T_CONFIG` description marks this operation unsupported in its stated scope; `GetStationInfo` nevertheless uses the type for reading. Check the instruction and CP-specific contract. |
| `IF_CONF_DNS` | 14 | Public record for two DNS IPv4 addresses, with the relevant CP 154x-1 restrictions for `T_CONFIG`. |
| `IP_V4` | 4 | Nested array/structure of four address octets. |
| `IP_V6` | 16 | Nested 16-byte IPv6 address structure in `IF_CONF_V6`. |

These are **published interface layouts**. `IF_CONF_HEADER` contains `FieldType : UINT` at 0–1, `FieldId : UINT` at 2–3, and `SubfieldCount : UINT` at 4–5. All other records listed below begin with `Id : UINT` at 0–1, `Length : UINT` at 2–3, and `Mode : UINT` at 4–5.

| Record | Payload byte positions | Record ID |
| --- | --- | --- |
| `IF_CONF_V4` | 6–9 `InterfaceAddress`; 10–13 `SubnetMask`; 14–17 `DefaultRouter`; each is `IP_V4`. | 30 |
| `IF_CONF_NOS` | 6–245 `NOS : ARRAY[1..240] OF BYTE`. | 40 |
| `IF_CONF_NTP` | 6–21 `NTP_IP : ARRAY[1..4] OF IP_V4`; address `i` starts at `6+4(i−1)`. | 17 |
| `IF_CONF_V6` | 6–21 `InterfaceAddress : IP_V6`. | 22 |
| `IF_CONF_MAC` | 6–11 `Mac : ARRAY[1..6] OF BYTE`. | 3 |
| `IF_CONF_DNS` | 6–9 `DNS_IP1 : IP_V4`; 10–13 `DNS_IP2 : IP_V4`. | 16 |

The IPv4 spans are four bytes each. The 240-byte station-name capacity ends at byte 245. A shorter active station name does not reduce the declared 246-byte capacity. `Mode` and permitted permanence depend on the instruction and device; a readable record is not proof that the same device allows that record to be written.

#### 9.3.1 CommConfig library UDTs

The V21 OUC catalog also defines the following nine `CommConfig` UDTs. They are provided by the CommConfig library; an arbitrary structure of the same width need not satisfy the required type identity. The V21 interface is listed for S7-1500 and G2. Per-type first TIA and firmware releases are not established here, so the V21 list must not be backdated to V13.

The table gives a **symbolic field contract**, with the storage width of each declared member. It deliberately does not turn those field widths into a universal UDT allocation: strings with odd capacity, arrays, enclosing structures, and optimized storage can add layout effects.

| Type | Size [B] | Published fields |
| --- | --- | --- |
| `Conf_Hostname` | Target/library allocation; field width 65 | Host-name member: `STRING[63]`. |
| `Conf_Domainname` | Target/library allocation; field width 254 | Domain-name member: `STRING[252]`. |
| `Conf_ClientId` | Target/library allocation; field width 256 | `ClientId : STRING[254]`. |
| `Conf_ClientId_Opaque` | Target/library allocation; fields 1 and 254 | `Length : BYTE`; `ClientId : ARRAY[0..253] OF BYTE`. |
| `Conf_DNS` | Target/library allocation; count 2, each string 41 | `Count : UINT`; `DNS_Server : ARRAY[1..4] OF STRING[39]`. |
| `Conf_DNS_IPV4` | Target/library allocation; fields 2 and 16 | `Count : UINT`; `DNS_Server : ARRAY[1..4] OF IP_V4`. |
| `Conf_IPSuitev4` | Target/library allocation; fields 2, 17, 2, 17 | `write_mode : UINT`; `IPAddress : STRING[15]`; `SubnetMaskPrefix : UINT`; `Gateway : STRING[15]`. |
| `Conf_IPSuitev4_IPV4` | Target/library allocation; fields 2, 4, 4, 4 | `write_mode : UINT`; `IPAddress`, `SubnetMask`, and `Gateway`, each `IP_V4`. |
| `Conf_NTP` | Target/library allocation; count 2, each string 256 | `Count : UINT`; `NTP_Server : ARRAY[1..4] OF STRING[254]`. |

The string-based DNS and NTP interfaces still support IPv4 addresses in the documented definition. Their string capacities are not the width of an IP address on the wire. `Conf_NTP` changes server addresses; the documented read operation for this UDT is unsupported. `CommConfig` is the instruction name, not an additional scalar data type.

### 9.4 Email configuration structures

The structure passed to `TMAIL_C` must match the mail-server address format, security requirements, CPU/CP capabilities, and instruction version.

| Type | Size [B] | Role |
| --- | --- | --- |
| `TMAIL_V4` | Not confirmed; depends on the SDT and version | Mail-connection configuration using an IPv4 server address. |
| `TMAIL_V6` | Not confirmed; depends on the SDT and version | Mail-connection configuration using an IPv6 server address. |
| `TMAIL_FQDN` | Not confirmed; depends on the SDT and version | Mail-connection configuration using a fully qualified domain name. |
| `TMAIL_V4_SEC` | Not confirmed; depends on the SDT and version | Secure mail-connection configuration using an IPv4 server address. |
| `TMAIL_V6_SEC` | Not confirmed; depends on the SDT and version | Secure mail-connection configuration using an IPv6 server address. |
| `TMAIL_QDN_SEC` | Not confirmed; depends on the SDT and version | Secure mail-connection configuration using a qualified domain name. |
| `TMail_FileReference` | 512 in the derived standard layout | File-attachment descriptor with two `STRING[254]` members. Available with TIA V16 / `TMAIL_C` V6.0, classic S7-1200 FW 4.4 and S7-1500 FW 2.8; other targets require their supported instruction version. |

The public fields of `TMail_FileReference` are `DirectoryPath` and `FileName`, in that order. Each occupies 256 bytes including its two-byte SIMATIC string header. In the **derived standard layout**, `DirectoryPath` occupies bytes 0–255 and `FileName` bytes 256–511. These capacities do not remove the instruction's path and file-name restrictions. This descriptor contains names of an attachment; it does not contain the attachment's file data.

### 9.5 OPC UA client structures

The OPC UA client instruction set applies to the supported S7-1500 targets; the `OPC_UA_Connect` applicability threshold is FW 2.6 or later. The classic S7-1200 and S7-1200 G2 do not have functional OPC UA client support in the family scope covered here. Their OPC UA server capabilities do not establish client support. A type name appearing in a shared interface listing also does not establish that it is a usable client feature on G2.

| Type | Size [B] | Role |
| --- | --- | --- |
| `OPC_UA_SessionConnectInfo` | Not confirmed; depends on the SDT and version | Configuration information for establishing the client's OPC UA session. |
| `OPC_UA_NodeId` | Not confirmed; depends on the SDT and version | Identifier of an OPC UA node. This identifies a node in the OPC UA address space rather than a PLC hardware object or data block. |
| `OPC_UA_NodeAdditionalInfo` | Not confirmed; depends on the SDT and version | Additional information associated with a node for the corresponding client operation. |
| `OPC_UA_NodeAdditionalInfoExt` | Not confirmed; depends on the SDT and version | Extended additional node-information structure used where the client interface requires that variant. |
| `OPC_UA_BrowsePath` | Not confirmed; depends on the SDT and version | Description of a browse path used to locate a node through the server's address-space relationships. |
| `OPC_UA_RelativePath` | Not confirmed; depends on the SDT and version | Relative-path description used as part of OPC UA path-based node navigation. |
| `OPC_UA_RelativePathElement` | Not confirmed; depends on the SDT and version | One element of a relative path; the enclosing path provides the complete sequence. |
| `OPC_UA_QualifiedName` | Not confirmed; depends on the SDT and version | Name qualified by an OPC UA namespace. A qualified name and a node identifier have different roles. |

Standalone declaration of an otherwise unused client SDT on a target that lists it is a separate target/compiler question; it does not provide OPC UA client functionality. Protocol support for a data representation also does not introduce a corresponding general PLC declaration type. OPC UA interfaces have their own restrictions on array shape, structures, lengths, and identifiers.

#### 9.5.1 OPC UA server mapping components

TIA V17 with S7-1500 FW 2.9 introduced the documented mappings for OPC UA `LocalizedText` and `ByteString`. These are complete SIMATIC structures generated or declared for an OPC UA server interface. Their members are not general-purpose standalone scalar types.

| Component type | Size [B] | Public field contract and restrictions |
| --- | --- | --- |
| `OPC_UA_LocalizedTextEncodingMask` | PLC allocation not independently established here | First member, `Encoding`, of the `LocalizedText` mapping. Bit 0 says that the locale is present; bit 1 says that the text is present. Value 3 sets both. |
| `OPC_UA_ByteStringActualLength` | PLC allocation not independently established here | First member, `ActualLength`, of the `ByteString` mapping; identifies the occupied portion of the following byte array. |

The `LocalizedText` member order is encoding mask, locale as `WSTRING`, then text as `WSTRING`. The `ByteString` member order is actual length followed by `ARRAY OF BYTE`. The configured string/array capacities and target layout determine their total sizes. API numeric ranges alone do not prove the PLC allocation or member offsets.

### 9.6 Diagnostic and identification structures

| Type | Size [B] | Role and conditions |
| --- | --- | --- |
| `DIS` | 14 in the derived standard layout; optimized allocation not asserted | Diagnostic-result SDT for `GET_DIAG` mode 1. |
| `DNN` | 6 in the derived standard layout; optimized allocation not asserted | Diagnostic-result SDT for `GET_DIAG` mode 2. |
| `GEOADDR` | 12 for its six `UINT` fields | Geographical-address structure used with `GEO2LOG` and `LOG2GEO`. It contains `HWTYPE`, `AREA`, `IOSYSTEM`, `STATION`, `SLOT`, and `SUBSLOT`, each of type `UINT`. Allocation of the containing block can add separate layout effects. |
| `IM0_Data` | Not confirmed; depends on the SDT definition | I&M0 identification-data structure used with `Get_IM_Data`. |
| `AlarmData` | Depends on the PLC data type definition and block layout | PLC data type described for the `Get_Alarm` interface. Its role in that interface does not make it a universally built-in SDT with one fixed layout. |

`IM1_Data`, `IM2_Data`, `IM3_Data`, and `IM4_Data` are not established here as declaration types; the existence of `IM0_Data` does not establish those additional names. An instruction parameter declared as `VARIANT` can still require an actual variable of one specific diagnostic SDT. The generic formal parameter and the required actual structure are separate constraints.

#### 9.6.1 Public geographic and diagnostic fields

The following offsets are **derived standard layouts** from the published member order and elementary widths. Their use does not establish fixed offsets inside an optimized DB.

| Type | Relative bytes and fields |
| --- | --- |
| `GEOADDR` | 0–1 `HWTYPE`; 2–3 `AREA`; 4–5 `IOSYSTEM`; 6–7 `STATION`; 8–9 `SLOT`; 10–11 `SUBSLOT`; every member is `UINT`. |
| `DIS` | 0–3 `MaintenanceState : DWORD`; 4–7 `ComponentStateDetail : DWORD`; 8–9 `OwnState : UINT`; 10–11 `IOState : WORD`; 12–13 `OperatingState : UINT`. |
| `DNN` | 0–1 `SubordinateState : UINT`; 2–3 `SubordinateIOState : WORD`; 4–5 `DNNmode : WORD`. |

`GEOADDR` expresses separate geographic coordinates. `HWTYPE` selects the object level and `AREA` selects the bus/system class; it is not a byte overlay of a hardware identifier.

The following `ComponentStateDetail` bit assignments follow the general TIA V20 `GET_DIAG` interface description. They are not established as invariant across all classic S7-1200 firmware documentation. The standard byte position of bit *k* in its `DWORD` is relative byte `4 + 3 − floor(k/8)`, bit `k mod 8`.

| Bits | Meaning |
| --- | --- |
| 0–2 | Additional-information enumeration. |
| 3 | Qualified diagnostics present. |
| 4 / 5 / 6 | Maintenance required / demanded / error. |
| 7–10 | Availability/ownership enumeration. |
| 11–14 | Configured-versus-present element enumeration. |
| 15 | Reserved; specified as 1. |
| 16 / 17 | Disabled / configuration-in-RUN active. |
| 18 / 19 | Input / output unavailable. |
| 20 / 21 / 22 | Diagnostic-buffer overflow / diagnostics unavailable / partial device failure. |
| 23–31 | Reserved; specified as 0. |

The classic S7-1200 V4.6 system manual publishes a different interpretation: bits 7–10 and 15 are reserved at zero; bits 11–14 are individual PNIO flags for correct submodule, replacement module, incorrect module, and disconnected module respectively; bits 22–31 are reserved at zero. The precise target or firmware boundary between these published descriptions is unresolved. Use the instruction contract applicable to the selected CPU. These semantic differences do not change the DWORD width or the derived DIS field offsets.

`IOState` bits 0–7 respectively represent good, disabled, maintenance required, maintenance demanded, error, unreachable, qualified diagnostics, and unavailable I/O. Bits 8–14 are reserved. In the documented S7-1500 interface, bit 15 distinguishes network/hardware faults when bit 4 or 5 is set; classic S7-1200 reserves bit 15. `DNNmode` bit 0 indicates diagnostics disabled when 1; bits 1–15 are reserved. `MaintenanceState`, `OwnState`, `OperatingState`, and `SubordinateState` are enumerations, not independent Boolean flags. Interpret enumeration values with the selected CPU's `GET_DIAG` version.

#### 9.6.2 Identification strings and raw I&M records

The public `IM0_Data` field contract contains `Manufacturer_ID : UINT`, `Order_ID : STRING[20]`, `Serial_Number : STRING[16]`, `Hardware_Revision : UINT`, a four-byte `Software_Revision` structure, three further `UINT` fields (`Revision_Counter`, `Profile_ID`, `Profile_Specific_Type`), and two `WORD` fields (`IM_Version`, `I&M_Supported`). `Software_Revision` contains one `CHAR` and three `USINT` members.

An ordinary SIMATIC `STRING[20]` occupies 22 bytes and `STRING[16]` occupies 18 bytes. The resulting declared standard field widths sum to 58 bytes. Siemens' I&M table also gives 20 and 16 in its byte-count column, corresponding to the character capacities and producing a 54-byte record-data sum. These are different quantities. The raw I&M byte-record layout must not be inferred from the PLC string declaration, and this reference does not treat 54 or 58 as an independently verified universal allocation of the actual SDT.

### 9.7 Organization-block start information

`RD_SINFO` uses system structures appropriate to the OB/event context. `TOP_SI` and `START_UP_SI` are interface parameter names, not additional data types. The following 25 `SI_*` types are distinct structure variants. There is no common byte size for the whole family; only the explicitly described public layouts below are expanded here.

| Type | Size [B] | Information context |
| --- | --- | --- |
| `SI_classic` | 12 in the derived standard layout | Classic-format start-information variant accepted by the applicable interface. The classic S7-1200 `RD_SINFO` interface rejects this variant. |
| `SI_none` | 4 in the derived standard layout | Header-only start-information variant. Its name does not mean zero-byte storage. |
| `SI_ProgramCycle` | Not confirmed; variant-specific | Program-cycle OB start information. |
| `SI_TimeOfDay` | Not confirmed; variant-specific | Time-of-day interrupt start information. |
| `SI_Delay` | Not confirmed; variant-specific | Time-delay interrupt start information. |
| `SI_Cyclic` | Not confirmed; variant-specific | Cyclic-interrupt start information. |
| `SI_HWInterrupt` | Not confirmed; variant-specific | Hardware-interrupt start information in the applicable standard variant. |
| `SI_HWInterrupt_Extended` | Not confirmed; variant-specific | Extended hardware-interrupt start-information variant. The required interface determines whether this or `SI_HWInterrupt` is appropriate. |
| `SI_Submodule` | Not confirmed; variant-specific | Submodule-related start-information variant. |
| `SI_SynchCycle` | Not confirmed; variant-specific | Synchronous-cycle start information for the corresponding supported OB context. |
| `SI_IORedundancyError` | Not confirmed; variant-specific | I/O-redundancy-error start information. |
| `SI_CPURedundancyError` | Not confirmed; variant-specific | CPU-redundancy-error start information. |
| `SI_TimeError` | Not confirmed; variant-specific | Time-error start information. |
| `SI_DiagnosticInterrupt` | Not confirmed; variant-specific | Diagnostic-interrupt start information. |
| `SI_PlugPullModule` | Not confirmed; variant-specific | Start information for a module insertion/removal event. |
| `SI_AccessError` | Not confirmed; variant-specific | Access-error start-information variant for the corresponding error context. |
| `SI_StationFailure` | Not confirmed; variant-specific | Station-failure start information. |
| `SI_Servo` | Not confirmed; variant-specific | Servo-cycle OB start information on CPUs supporting that Motion execution context. |
| `SI_PreOrPostServo` | Not confirmed; variant-specific | Start information for an OB executed before or after the servo processing context. |
| `SI_Ipo` | Not confirmed; variant-specific | Interpolator-related start information for the corresponding Motion OB context. |
| `SI_PreInterpolator` | Not confirmed; variant-specific | Start information for the pre-interpolator OB context. |
| `SI_Startup` | Not confirmed; variant-specific | CPU/startup OB start information. |
| `SI_ProgIOAccessError` | Not confirmed; variant-specific | Program I/O-access-error start-information variant. |
| `SI_CPURedundancyError_V2` | Not confirmed; variant-specific | Version 2 CPU-redundancy-error information structure. It has its own structure contract and must not be substituted solely on the basis of the shared family name. |
| `SI_IORedundancyError_V2` | Not confirmed; variant-specific | Version 2 I/O-redundancy-error information structure with its own structure contract. |

The `START_UP_SI` interface includes `SI_classic`, `SI_none`, and `SI_Startup`. Availability of every variant is conditional on the CPU's supported OBs and functions. A Servo-related structure name does not imply ServoOB support on every S7-1200. The list is a TIA V21 type set; separate historical minimum firmware versions are not assigned to every entry.

| Type | Public member order and derived standard offsets |
| --- | --- |
| `SI_classic` | Byte 0 `EV_CLASS : BYTE`; 1 `EV_NUM : BYTE`; 2 `PRIORITY : BYTE`; 3 `NUM : BYTE`; 4 `TYP2_3 : BYTE`; 5 `TYP1 : BYTE`; 6–7 `ZI1 : WORD`; 8–11 `ZI2_3 : DWORD`. In `EV_CLASS`, bits 0–3 carry the event ID and bits 4–7 the event class. |
| `SI_none` | Byte 0 `SI_Format : USINT`; 1 `OB_Class : USINT`; 2–3 `OB_Nr : UINT`. |

For `SI_ProgramCycle`, the public header fields `SI_Format`, `OB_Class`, and `OB_Nr` are followed by `Initial_Call : BOOL` and `Remanence : BOOL`. This is a symbolic field contract, not a claim about the generated optimized layout of every start-information variant. The classic S7-1200 manual specifically documents errors 16#8081 and 16#8083 when `SI_classic` is used at `TOP_SI` and `START_UP_SI` respectively.

### 9.8 Motion Control technology-object types

Technology-object types describe interfaces to configured technology objects and their associated data. Generic categories such as `TO_Object` and `TO_Axis` can have polymorphic meaning in instruction interfaces. Matching their visible fields with a manually created structure does not produce an equivalent technology object.

| Type | Size [B] | Role and scope |
| --- | --- | --- |
| `TO_Object` | Depends on object, technology version, and configuration | General technology-object interface category. |
| `TO_Axis` | Depends on object, technology version, and configuration | General axis interface category used where the instruction accepts the appropriate supported axis types. |
| `TO_SpeedAxis` | Depends on object, technology version, and configuration | Technology object for a speed-controlled axis. |
| `TO_PositioningAxis` | Depends on object, technology version, and configuration | Technology object for a positioning axis. It also occurs in the classic S7-1200 mechanism for passing technology objects to blocks; the passing mechanism does not define the complete object-data size. |
| `TO_SynchronousAxis` | Depends on object, technology version, and configuration | Axis technology object supporting the applicable synchronous-motion functions. |
| `TO_ExternalEncoder` | Depends on object, technology version, and configuration | Technology object representing an external encoder. |
| `TO_OutputCam` | Depends on object, technology version, and configuration | Output-cam technology object for a configured cam-switching function. |
| `TO_CamTrack` | Depends on object, technology version, and configuration | Technology object for a track of output cams. |
| `TO_MeasuringInput` | Depends on object, technology version, and configuration | Technology object for a measuring-input function. |
| `TO_CamBase` | Depends on object, technology version, and configuration | Common cam-object interface category in the applicable T/TF technology functions. |
| `TO_Cam` | Depends on object, technology version, and configuration | Cam technology object used by the applicable T/TF Motion functions. |
| `TO_Cam_10k` | Depends on object, technology version, and configuration | Specific cam-capacity variant in the T/TF cam family. The suffix does not give its allocated byte size. |
| `TO_Cam_600Seg` | Depends on object, technology version, and configuration | Specific segmented-cam variant in the T/TF cam family. |
| `TO_Cam_6kSeg` | Depends on object, technology version, and configuration | Another segmented-cam capacity variant in the T/TF cam family. |
| `TO_Kinematics` | Depends on object, technology version, and configuration | Kinematics technology object for supported T/TF coordinated-motion functions. |
| `TO_LeadingAxisProxy` | Depends on object, technology version, and configuration | Leading-axis proxy technology object in the applicable T/TF synchronization functions. |
| `TO_Interpreter` | Depends on object, technology version, and configuration | Interpreter technology object in the supported T/TF Motion subsystem. |
| `TO_InterpreterProgram` | Depends on object, technology version, and configuration | Technology-object type associated with an interpreter program. |
| `TO_InterpreterMapping` | Depends on object, technology version, and configuration | Technology-object type associated with interpreter mapping. |
| `TO_CommandTable` | Depends on object, technology version, and configuration | Command-table technology-object type in the classic S7-1200 mechanism for passing technology objects. |

These names belong to the Motion V10/TIA V21 scope and applicable target-specific variants; they are not all assigned to the earliest TIA releases. The technology version must be compatible with the target firmware:

| S7-1500 firmware | Motion technology version |
| --- | --- |
| 2.0 / 2.1 | 3 |
| 2.5 / 2.6 | 4 |
| 2.8 | 5 |
| 2.9 | 6 |
| 3.0 | 7 |
| 3.1 | 8 |
| 4.0 | 9 |
| 4.1 | 10 |

CPU-model exceptions remain applicable. This mapping selects the Motion subsystem generation; it does not independently establish the first release of each `TO_*` type or the version of each `MC_*` instruction instance.

### 9.9 Motion telegram structures

| Type or naming pattern | Size [B] | Role |
| --- | --- | --- |
| `PD_TELx` | Depends on the selected telegram definition | Naming pattern for process-data telegram structures. Replace `x` with the supported telegram number; `PD_TELx` is not a literal declaration type ending in the letter `x`. |
| `DX_TEL_SyncOp` | Not confirmed; depends on the structure definition | Telegram structure for the applicable synchronous-operation interface. |
| `PD_STW1_611Umode` | Not confirmed; depends on the structure definition | Control-word-1 structure for the corresponding 611U-mode telegram interface. |
| `PD_STW2_611Umode` | Not confirmed; depends on the structure definition | Control-word-2 structure for the corresponding 611U-mode telegram interface. |
| `PD_ZSW1_611Umode` | Not confirmed; depends on the structure definition | Status-word-1 structure for the corresponding 611U-mode telegram interface. |
| `PD_ZSW2_611Umode` | Not confirmed; depends on the structure definition | Status-word-2 structure for the corresponding 611U-mode telegram interface. |

The structure required by a telegram interface is determined by that interface. A familiar control-word or status-word name does not justify assigning a complete structure a guessed size.

`PD_TEL3` is one concrete member of the `PD_TELx` family. The current documented drive/encoder connection uses the complete telegram structure containing `Input` and `Output`; a nested name such as `PD_TEL3_IN` is not thereby supported as an independent DB declaration for that connection. Classic S7-1200 telegram migration from Motion V6 to V7 also changed nested naming and at least one status-bit meaning. A telegram number without its technology version is therefore insufficient for a dependable field or bit map.

### 9.10 Counting and measurement types

| Type | Size [B] | Role and target conditions |
| --- | --- | --- |
| `HSC_Count` | Not confirmed; depends on the SDT definition | Count-related SDT for `CTRL_HSC_EXT` on classic S7-1200. |
| `HSC_Period` | 12 | Period-measurement SDT for `CTRL_HSC_EXT` on classic S7-1200. |
| `HSC_Frequency` | 8 in the derived standard layout; actual allocation requires its SDT context | Frequency-measurement SDT for `CTRL_HSC_EXT` on classic S7-1200. |
| `High_Speed_Counter` | Depends on the technology object, instance, and version | S7-1500 high-speed-counter technology-object/instruction family and its corresponding instance data. |
| `SSI_Absolute_Encoder` | Depends on the technology object, instance, and version | S7-1500 technology-object/instruction family for an SSI absolute encoder. The CPU1515R R/H functionality includes this object from FW 2.9. |

`HW_HSC`, `HSC_Count`, and `High_Speed_Counter` are three different categories: hardware identifier, instruction data structure, and technology-object/instance family respectively. Their similar names do not make their declarations or memory allocations interchangeable.

The classic S7-1200 public measurement fields support these **derived standard layouts**:

| Type | Relative bytes/bits and fields |
| --- | --- |
| `HSC_Period` | 0–3 `ElapsedTime : UDINT`; 4–7 `EdgeCount : UDINT`; 8.0 `EnHSC : BOOL`; 8.1 `EnPeriod : BOOL`; 8.2–8.7 unused; 9 alignment; 10–11 `NewPeriod : INT`. The resulting 12-byte layout agrees with the generic SDT length. |
| `HSC_Frequency` | 0–3 `Frequency : DINT`; 4.0 `EnHSC : BOOL`; 4.1 `EnPeriod : BOOL`; 4.2–4.7 unused; 5 alignment; 6–7 `NewPeriod : INT`. |

`HSC_Count` exposes the `DINT` values `CurrentCount` and `CapturedCount`, status and enable `BOOL` fields, four `INT` configuration members, and six `DINT` new count/limit/reference values. The exact SDT definition must be used for their offsets; this description does not flatten its input/output groupings into a purported universal allocation. `ElapsedTime` is a nanosecond measurement, `Frequency` is signed hertz, and `NewPeriod` selects the documented millisecond measurement interval. Their numeric units are not new scalar types.

### 9.11 PID controllers and supporting instances

Controller names denote versioned instructions, technology objects where provided, and their associated instances. Their instance sizes include state and depend on the selected version. A controller name does not define a scalar value type.

| Type or instance family | Size [B] | Role |
| --- | --- | --- |
| `PID_Compact` | Depends on controller version and instance | Compact PID controller family with a versioned instance and corresponding technology configuration. |
| `PID_3Step` | Depends on controller version and instance | Three-step controller family for the applicable actuator-control arrangement. |
| `PID_Temp` | Depends on controller version and instance | Temperature-control PID family with version-specific instance data and supported functions. |
| `CONT_C` | Depends on FB and instance version | Standard continuous-controller block family. |
| `CONT_S` | Depends on FB and instance version | Standard step-controller block family. |
| `TCONT_CP` | Depends on FB and instance version | Temperature-controller block family for the corresponding continuous/pulse-control interface. |
| `TCONT_S` | Depends on FB and instance version | Temperature step-controller block family. |
| `PULSEGEN` | Depends on FB and instance version | Pulse-generator block family used with the corresponding control application. It has no technology object in the controller-family classification. |
| `Polyline` | Depends on FB and instance version | Auxiliary instruction/instance family for a piecewise-linear characteristic. |
| `SplitRange` | Depends on FB and instance version | Auxiliary family for dividing a control value into the corresponding output ranges. |
| `RampFunction` | Depends on FB and instance version | Auxiliary ramp-generation instruction/instance family. |
| `RampSoak` | Depends on FB and instance version | Auxiliary family for a ramp-and-hold profile. Its profile data and execution state belong to the particular instruction interface. |
| `Filter_PT1` | Depends on FB and instance version | Auxiliary first-order lag-filter family. |
| `Filter_PT2` | Depends on FB and instance version | Auxiliary second-order lag-filter family. |
| `Filter_DT1` | Depends on FB and instance version | Auxiliary differentiating-filter family with a first-order lag. |
| `Filter_Universal` | Depends on FB and instance version | Auxiliary universal-filter instruction/instance family using the supported configuration of that version. |

The supporting controller and filter families remain conditional on CPU and instruction availability. The following tables select compatible versions of the three principal PID instructions. Values in the PID columns are **instruction versions**, not firmware numbers. A dash means that no supported entry is specified in this compatibility set.

| CPU and firmware | `PID_Compact` versions | `PID_3Step` versions | `PID_Temp` versions |
| --- | --- | --- | --- |
| Classic S7-1200 2.x / 3.x | 1.1, 1.2 | 1.0, 1.1 | — |
| Classic S7-1200 4.0 | 1.2, 2.2 | 1.1, 2.2 | — |
| Classic S7-1200 4.1 | 1.2, 2.2 | 1.1, 2.2 | 1.0 |
| Classic S7-1200 ≥ 4.2 | 1.2, 2.2, 2.3 | 1.1, 2.2, 2.3 | 1.0, 1.1 |
| S7-1200 G2 1.0…4.0 compatibility range | 3.0 | 2.3 | 1.1 |
| S7-1200 G2 ≥ 4.1 | 3.0 | 2.3 | 1.1, 2.0 |

The G2 `1.0…4.0` entry expresses the compatibility range. It does not imply that every intervening firmware number, including G2 4.0, was released.

| S7-1500 firmware | `PID_Compact` versions | `PID_3Step` versions | `PID_Temp` versions |
| --- | --- | --- | --- |
| 1.0 | 2.0 | 2.0 | — |
| 1.1 | 2.0, 2.1 | 2.0, 2.1 | — |
| 1.5…1.6 | 2.0, 2.1, 2.2 | 2.0, 2.1, 2.2 | — |
| 1.7…1.8 | 2.0, 2.1, 2.2 | 2.0, 2.1, 2.2 | 1.0 |
| 2.0 / 2.1 | 2.0, 2.1, 2.2, 2.3 | 2.0, 2.1, 2.2, 2.3 | 1.0, 1.1 |
| 2.5…2.9 | 2.0, 2.1, 2.2, 2.3, 2.4 | 2.0, 2.1, 2.2, 2.3 | 1.0, 1.1 |
| 3.0 | 2.4 | 2.3 | 1.1 |
| 3.1…4.0 | 2.4, 3.0 | 2.3 | 1.1 |
| ≥ 4.1 | 2.4, 3.0 | 2.3 | 1.1, 2.0 |

The firmware and instruction version form a compatibility pair. A newer firmware does not necessarily support all older instruction versions; for example, the FW 3.0 row contains a narrower selection than the preceding S7-1500 rows.

### 9.12 Kinematics and interpreter parameter structures

| Type | Size [B] | Role and conditions |
| --- | --- | --- |
| `TO_Struct_TransformationParameter_V1` | Not confirmed; depends on the SDT definition | Transformation-parameter structure present in kinematics V5/TIA V16. An earlier first release is not specified. The `V1` suffix selects this structure version. |
| `TO_Struct_TransformationParameter_V2` | Not confirmed; depends on the SDT definition | Version 2 transformation-parameter SDT, used among other contexts in `MC_Transformation` OB98. This is parameter data rather than a standalone technology object. |
| `TO_Struct_Kinematics_Frame` | Not confirmed; depends on the SDT definition | Structure of the `Frame` parameter for `MC_DefineWorkspaceZone`. Its use is tied to that kinematics interface. |
| `TO_Struct_Kinematics_KinematicsFrame` | Not confirmed; depends on the technology version | Frame parameter type of `MC_DefineTool`, also listed for interpreter mapping. Its existence does not establish an interchangeable binary layout with every other frame type. |
| `TO_Struct_Ipr_Position` | 48 declared field bytes | Interpreter position structure: six `LREAL` members, `x`, `y`, `z`, `A`, `B`, `C`. |
| `TO_Struct_Ipr_AxPosition` | 48 declared field bytes | Interpreter axis-position structure: six `LREAL` members, `a1` through `a6`. |
| `TO_Struct_Ipr_JtPosition` | 48 declared field bytes | Interpreter joint-position structure: six `LREAL` members, `j1` through `j6`. |
| `TO_Struct_Ipr_Frame` | 48 declared field bytes | Interpreter frame structure: six `LREAL` members, `x`, `y`, `z`, `A`, `B`, `C`. |

The four `TO_Struct_Ipr_*` contracts belong to the supported S7-1500T interpreter context and its PLC-to-MCL mapping. Six consecutive `LREAL` fields give derived standard byte ranges 0–7, 8–15, 16–23, 24–31, 32–39, and 40–47. This calculation does not specify an interpreter technology object's allocation. A user UDT with the same fields is not automatically accepted in interpreter mapping. MCL also has its own language-specific type categories; its entire type catalog must not be treated as general STEP 7 PLC declaration syntax.

Technology objects contain further nested `TO_Struct_*` and versioned transformation structures. Their complete member catalogs belong to the selected technology version. The table covers the named public parameter and mapping structures identified here, not every nested member type generated by every technology package.

### 9.13 SIMATIC Ident, specialized options, and R/H distinctions

| Type or category name | Size [B] | Role and declaration scope |
| --- | --- | --- |
| `TO_IDENT` | Depends on the type, version, and instance | Parameter type of the applicable SIMATIC Ident instructions. Use the exact type required by that interface. |
| `TO_TAGLAYOUT` | Depends on version and generated tag layout | Parameter type used for SIMATIC Ident tag-layout data. The configured tag layout also generates a type whose name depends on the project name. |
| `TO_TagFields` | Not confirmed; API category does not specify PLC allocation | Name in the Openness API's type categories. It is not established as an interchangeable PLC declaration spelling for `TO_TAGLAYOUT`. |
| `TO_BasicPos` | Depends on instance, technology version, and configuration | Basic-positioner technology-object/instruction family. The CPU1515R R/H functionality includes it from FW 2.9. Its supported technology interface does not establish arbitrary standalone declaration contexts. |
| `TO_Ident` | Depends on technology configuration and instance | Spelling used for the Ident technology functionality in the CPU1515R R/H version scope, available from FW 4.1. This is a feature-availability statement; capitalization alone does not establish a second PLC data type distinct from `TO_IDENT`. |
| `TUN_EC`, `TUN_ES`, `PID_CP`, `PID_ES` | Not confirmed; option and instance dependent | Specialized S7-300/400 controller-family categories in the Openness API. Their availability belongs to the corresponding option or technology package; they are not established as built-in declaration names on every S7-300. |
| `AXIS_REF` | Not confirmed; technology-specific representation | EMC axis-object category in the Openness API. It belongs to that technology's object/interface model and does not establish a general S7-300 reference declaration. |

S7-1500R/H has its own functional version conditions. In the CPU1515R scope, `TO_BasicPos` and `SSI_Absolute_Encoder` appear from FW 2.9, and `TO_Ident` from FW 4.1. The dynamic/data-block operations `CREATE_DB`, `READ_DBL`, `WRIT_DBL`, and `DELETE_DB` are available there from FW 3.0; these names denote operations, not additional PLC data types. Version thresholds for standard S7-1500 CPUs do not automatically apply to R/H models.

## 10. Data types in fail-safe programs

An F-CPU can execute both a standard program and a fail-safe program. The fail-safe program has its own restricted type set. Support for a type in the standard program does not make it valid in an F-FB, F-FC or F-DB. The F-program described here is programmed in LAD/FBD.

### 10.1 Elementary F-program types

| Type | Size [B] | Value and logical bit representation | S7-300 F | Classic S7-1200 F | S7-1200 G2 F | S7-1500 F |
|---|---|---|---|---|---|---|
| `BOOL` | One logical bit; allocation depends on context | `FALSE` or `TRUE` | Yes | Yes | Yes | Yes |
| `INT` | 2 | Signed integer: −32 768…32 767; 16-bit two's complement; bit 15 is the sign bit | Yes | Yes | Yes | Yes |
| `WORD` | 2 | Bits 15…0 form a 16-bit string; no sign or exponent field | Yes | Yes | Yes | Yes |
| `DINT` | 4 | Signed integer: −2 147 483 648…2 147 483 647; 32-bit two's complement; bit 31 is the sign bit | Yes | Yes | Yes | Yes |
| `TIME` | 4 | Signed 32-bit count of milliseconds in two's complement; bit 31 is the sign bit | Yes | Yes | Yes | Yes |
| `DWORD` | 4 | Bits 31…0 form a 32-bit string; no sign or exponent field | Yes | No | No | No |

`BOOL` represents logical conditions and binary states. `INT` and `DINT` represent signed integer values within their respective ranges. `WORD` and, where allowed, `DWORD` represent bit strings; their availability does not authorize declarations of the unsigned integer types `UINT` or `UDINT` in the F-program.

`TIME` has the representation range **−24 d 20 h 31 min 23 s 648 ms…+24 d 20 h 31 min 23 s 647 ms**. A particular fail-safe timing instruction can impose a narrower permissible range on an input parameter. A type's representation range and an instruction's permitted parameter values must therefore be checked separately.

The general F-program type set does not include user declarations of `BYTE`, `SINT`, `USINT`, `UINT`, `UDINT`, `LINT`, `ULINT`, `LWORD`, `REAL`, `LREAL`, `CHAR`, `WCHAR`, `STRING`, `WSTRING` or the other standard date/time types. Special system ports or diagnostic information do not expand that set.

The sizes above describe the user values. They do not express the total memory required for the compiled safety program or its internal state.

### 10.2 F-compliant PLC data types — F-UDTs

**Meaning:** a named structure created as an F-compliant PLC data type. It groups values that can be used in the fail-safe program and retains the type's identity across its uses.

**Size [B]:** depends on its fields and the resulting F-UDT memory layout. There is no fixed size for the category.

| Family | F-UDT availability |
|---|---|
| S7-300 F | No |
| Classic S7-1200 F | Conditional on the Safety package and target; available in Safety V13 SP1 configurations |
| S7-1200 G2 F | Conditional on the supported G2 Safety configuration |
| S7-1500 F | Conditional on the Safety package and target; available in Safety V13 configurations |

An ordinary anonymous `STRUCT` with identical fields is not a substitute for an F-compliant PLC data type. Fields must use permitted F-program types, and **`ARRAY` is not allowed inside an F-UDT**.

Nested F-UDTs are available from **Safety V17**. For **Safety V21**, the nesting limit is up to **8 levels**, reduced to **7 levels for a global F-DB**. F-FC call nesting and F-FB multi-instances can further reduce the available depth. The 26-level nesting capability of standard structures on newer CPU targets does not apply automatically to F-UDTs.

The listed early Safety releases establish available combinations; they do not establish a single earliest firmware requirement for every F-UDT configuration.

### 10.3 Fixed-size F arrays

**Types:** one-dimensional arrays of `INT` or `DINT`.

**Size [B]:** `2 × N` for the element data of an `INT` array, or `4 × N` for the element data of a `DINT` array. These expressions do not include the entire F-DB or safety-program overhead.

| Family | F-array availability |
|---|---|
| S7-300 F | No |
| Classic S7-1200 F | No |
| S7-1200 G2 F | Conditional; available in supported Safety V20 configurations |
| S7-1500 F | Conditional; Safety V15 and CPU FW ≥ 2.0 |

The lower array bound is **0** and the upper bound must be no greater than **10000**. At the upper limit, the declaration must be checked against the selected Safety release: descriptions of the bounds `0…10000` and of a maximum of 10000 elements are not fully consistent. For an accepted declaration `ARRAY[0..10000] OF INT`, the inclusive index count is 10001 and its value payload is 20002 bytes; the corresponding `DINT` payload is 40004 bytes. These arithmetic counts do not establish whether a particular earlier Safety compiler accepts that maximum declaration.

Reading is provided through `RD_ARRAY_I` for `INT` and `RD_ARRAY_DI` for `DINT`. For an out-of-range index, these instructions set the error indication and return element 0; that first element is used for the fail-safe substitute value. The specified F-DB and connection rules must be followed; a shortened reference to an F-DB does not by itself establish support for an instance F-DB. Arrays are not permitted as fields of F-UDTs.

Reading a standard DB array element with a constant index is a separate operation. It does not make F-array declarations available on S7-300 F or classic S7-1200 F.

### 10.4 Open-array F parameters — `ARRAY[*]`

**Meaning:** a formal `InOut` parameter of an F-FC or F-FB that accepts a compatible `INT` or `DINT` array with bounds supplied by the actual argument. It does not allocate a resizable global array.

**Size [B]:** the supplied array retains its own declared storage. No universal byte size for the parameter-passing overhead is established here.

This mechanism applies to supported S7-1500 F and S7-1200 G2 F configurations. For S7-1500 F, CPU FW ≥ 2.0 is required; support is established by Safety V16, while the exact first earlier package boundary between V15 and V15.1 remains unresolved. The G2 mechanism is present in Safety V20 configurations; the S7-1500 firmware number must not be reused as a G2 firmware requirement.

The formal port of a system array instruction may be described using `VARIANT`. That description does not permit user-written `VARIANT` declarations in an F-FB or F-DB.

### 10.5 Named value types in the S7-1500 F-program

**Meaning:** a named type based on a permitted integer or bit-string type, with names assigned to selected values.

**Available base types and sizes:**

| Base type | Size [B] |
|---|---:|
| `INT` | 2 |
| `WORD` | 2 |
| `DINT` | 4 |

The permitted bases follow from the intersection of the general NVT base-type set and the S7-1500 F-program type set. `BOOL` is not a standard NVT base type. `TIME` is excluded, and `DWORD` is not an S7-1500 F-program type.

F-program NVTs are available with **TIA/Safety V19** and require the software-unit environment. The Safety Unit infrastructure is available from **Safety V18 and S7-1500 F FW ≥ 2.6**, but that infrastructure requirement alone does not establish NVT support for every FW 2.6 target. The exact target and compiler combination must support NVT declarations.

This category is not available for S7-300 F, classic S7-1200 F or S7-1200 G2 F. NVTs retain the semantics of their base type; assigning names to values does not create a closed enumeration that permits only those named values.

Know-how protection and library use have their own release boundaries:

| TIA/Safety release | NVT use in F-blocks |
|---|---|
| V19 | Available in the supported Safety Unit context; not in know-how protected F-blocks. |
| V20 | Use in know-how protected safety programs is supported. Know-how protected safety programs in libraries do not yet support NVTs. |
| V21 | Support extends to know-how protected Safety programs in libraries and typified F-blocks. |

These changes affect legal use and distribution of the type. They do not change an NVT value's base-type width or add a per-value name field.


### 10.6 Context-specific F-data restrictions

Grouping F-I/O inputs and outputs in structures excludes fields such as `WORD`, `DWORD` and `TIME` in that particular use. This does not remove `WORD` or `TIME` from the general F-program type set.

Communication buffers between F-runtime groups have their own size and version requirements. They must not be treated as a universal F-UDT layout rule. A `BYTE` in non-fail-safe `DIAG` service information likewise does not authorize the type in user-written F logic.

User-written parameter declarations such as `ANY`, `BLOCK_FB` and `VARIANT` are not part of the general F-program type set. Instruction signatures must be interpreted separately from the types permitted in user declarations.

### 10.7 Value bytes and the compiled F-data layout

The elementary F types retain their value semantics. If an F result is transferred through a permitted interface to an explicitly defined standard-access exchange field, an `INT` or `WORD` occupies two value bytes, and a `DINT`, `DWORD` or `TIME` occupies four value bytes. The standard S7 field stores the most significant byte first. For example, an `INT` result of −2 has the 16-bit value pattern `16#FFFE`, corresponding to `FF FE` in that standard exchange field. This example is a value encoding, not an address map of an F-DB.

No fixed multiplier converts these user-value sizes into the total memory required by a compiled safety program. Generated F-blocks, their interfaces and the selected safety system determine the actual allocation. The public type name does not expose all generated implementation state.

For F-arrays, `N` values of `INT` contain `2 × N` value bytes and `N` values of `DINT` contain `4 × N` value bytes. Their logical element order follows the array index. The first element is index 0. These counts do not establish the absolute base address, the total F-DB allocation, or a raw-memory access permission. Access the array through the supported F-array instruction interface.

An F-UDT is laid out from its actual declared fields and nested F-UDTs. Its size is not the number of field names, and a sequence of `BOOL` members must not be assigned a universal packed-byte layout without the concrete compiled declaration. An F-program NVT has the logical value encoding of its permitted base type; its value names do not add a string, tag or enumeration table to every stored value.

**Slice access is not supported in the safety program.** A bit-field explanation therefore does not imply that expressions for selecting arbitrary bits, bytes or words are legal in F code. The standard and F programs must use their permitted data-exchange interfaces and access directions.

Structured F-I/O tags have additional address rules: separate structures start at multiples of 16 bits, including nested F-compliant structures; they contain only existing channels enabled in the hardware configuration and their permitted value-status fields. A structured tag must not mix standard I/O and F-I/O. With 1oo2 sensor evaluation, only channels providing the evaluation result may be included; a structured tag containing F outputs must not overlap another PLC tag. The I/O configuration supplies the actual channel byte and bit addresses. These process-image rules do not define the internal layout of every F-DB or F-UDT.

## 11. Memory representation, byte order, bit numbering and layout boundaries

### 11.1 What a size or byte map measures

A data type's value width, its allocated memory span, a parameter descriptor and the object referenced by that descriptor are different quantities.

| Quantity | Meaning | Example |
|---|---|---|
| Logical value width | Number of meaningful bits in the value | BOOL has two values and one logical bit. |
| Representation size | Bytes encoding that value or documented descriptor | INT has a 2 B representation; a classic ANY descriptor is 10 B. |
| Allocated span | Space consumed in a particular containing layout, including required alignment and padding | A current standard ARRAY of three BYTE elements occupies a 4 B span. |
| Referenced-object size | Storage occupied by the object being addressed | A 10 B ANY can describe three INT elements occupying 6 B. |
| Compiler or system overhead | Call metadata, instance administration, reserve, or internal structures not specified by the value encoding | VARIANT's published length of 0 B does not mean that its mechanism consumes no CPU memory. |

For each byte map, B0 is the first byte of the stated representation, B1 the second, and so on. These are relative offsets. A map becomes an absolute DB layout only after the containing object's base address, alignment and access mode have been established. The elementary maps in this document describe the stated standard S7 representation; they do not expose the internal layout of optimized data or the complete backing storage of an F-program variable.

A fixed value encoding also does not describe load-memory block headers, executable code, protocol headers, HMI metadata, or the complete memory needed to execute an instruction.

### 11.2 Bit significance and byte addresses

Within one byte, b0 is the least significant bit and b7 the most significant. A physical byte.bit address such as DBX20.3 selects bit 3 of byte 20. The number after the point is a bit position from 0 to 7, not a decimal fraction.

In a multi-byte value, value bit b0 remains its least significant bit. Its physical byte depends on the representation's byte order.

| 32-bit value | B0 | B1 | B2 | B3 | Location of value bit b0 |
|---|---|---|---|---|---|
| Big-endian representation of 16#12345678 | 12 | 34 | 56 | 78 | B3.b0 |
| Little-endian representation of 16#12345678 | 78 | 56 | 34 | 12 | B0.b0 |

All byte entries in this table are hexadecimal. Big-endian changes the order of bytes, not the significance of bit numbers within each byte. If a standard S7 DWORD at DBD0 contains 16#00000001, its set bit is DBX3.0. DBX0.0 is a different bit of that four-byte value.

Consequently, “bit 0 of the value” and “bit 0 of the first addressed byte” are not interchangeable phrases. A bit-field description must state which it means. Similarly, a diagram drawn with the most significant bit on the left is not evidence about physical byte order.

### 11.3 Standard storage and optimized storage

| Family and access mode | Applicable interpretation |
|---|---|
| S7-300 standard storage | Traditional S7 addressable layout, with big-endian multi-byte scalar representation and the standard structure/array rules. |
| Classic S7-1200 standard storage | Standard S7-compatible representation for supported types; the declaration still has CPU and block-size restrictions. |
| S7-1200 G2 standard storage | Apply the documented standard representation and the configured target's admissible types; G2 is a separate target from the classic S7-1200. |
| S7-1500 standard storage | Traditional externally addressable S7 layout; native processor access can require byte rearrangement internally. |
| Classic S7-1200 optimized storage | Symbolic, compiler-selected storage. The documented optimized model packs Boolean data; the S7-1500 byte-per-BOOL rule must not be transferred to it. |
| S7-1500 optimized storage | Symbolic, processor-optimized storage. Siemens describes a little-endian processor model, alignment according to data size, and one-byte BOOL storage. These facts do not define a stable byte-for-byte application layout. |
| S7-1200 G2 optimized storage | Symbolic, target-specific storage. This document does not assign a universal raw scalar order, BOOL-array packing rule or member-offset formula without a target-specific layout contract. |

In optimized blocks, declaration order does not generally determine storage order. The compiler can arrange larger elements before smaller elements, align elements, handle nested structures as units, and separate retentive data. Download reserve and instance administration can add further allocation. A compact structure is therefore not guaranteed to have the same total size in every CPU family, firmware, compiler release or containing context.

The S7-1500 little-endian storage model does not authorize a six-byte POINTER or ten-byte ANY to inspect an optimized object. It also does not determine the byte order of a communication buffer. Program-level values remain the values of their declared types when the compiler changes their internal placement.

Where a published representation is explicitly opaque or unspecified, byte order is unspecified with it. Do not invent an address field, length field, type tag or null pattern for REF_TO, VARIANT, an open-array parameter, or a proprietary instruction instance.

### 11.4 Standard alignment, padding and element strides

Standard STRUCTs and arrays start at an even byte offset and include final padding to a word boundary. STRUCT members remain in declaration order. Consecutive BOOL fields can share a byte; BYTE and CHAR fields start at a byte boundary; other ordinary supported components follow their standard word-alignment rules. An embedded structure or array can therefore introduce padding before and after its own contents.

For example, three independent BYTE fields at offsets 0, 1 and 2 followed by an ARRAY of seven BYTE elements produce this layout:

| Relative bytes | Contents |
|---|---|
| B0–B2 | Three independent BYTE fields |
| B3 | Alignment padding before the array |
| B4–B10 | Seven array elements |
| B11 | Padding at the array end |
| B12 | A following independent BYTE field |

Thus, replacing a group of individual bytes with an array can change subsequent offsets even when the number of meaningful data bytes remains unchanged.

The stride of an ARRAY OF STRUCT or ARRAY OF UDT includes the padded size of each element. For multidimensional arrays, the last index changes fastest. Standard BOOL rows start at a new byte, so padding may occur between rows as well as at the overall array end. Any offset or allocation formula must respect these boundaries.

The current standard rules cannot be applied retroactively to every compiler output. In particular, TIA V15 changed S7-1500 BOOL/BYTE/CHAR array sizing so that standalone and structure-member contexts use consistent sizes. The compatibility description does not provide a universal formula for every earlier variant. An existing V13/V14 layout must be checked against its own generated offsets.

Padding is not a declared application value. It may be copied along with a structure when source and destination use the same layout rules, or left unchanged when the transfer converts members individually. Do not use padding as independent persistent state or assume that two equal typed values must have identical padding bytes. Historical changes in handling unused UDT bits provide an additional reason not to overlap unrelated tags with those bits.

### 11.5 Parameter transfer and instance memory

The word “pointer” in a description of parameter transfer does not necessarily mean a variable of type POINTER or ANY. The compiler also uses internal reference mechanisms when passing structured values.

For S7-1200/S7-1500, structured FC parameters are generally transferred by reference, whereas FB input and output structures are generally copied into or out of instance storage. Structured InOut parameters normally use reference transfer. If the actual object and the called block have different access modes, conversion can require a temporary copy instead. Additional local memory and a later copy-back may then be involved.

The formal type's size therefore cannot be used alone to calculate the peak call-stack requirement or the caller's complete instance allocation. A change from standard to optimized access may alter transfer behavior even when the type name and its logical contents remain the same.

An FB instance contains persistent working data associated with that FB and its version. An FB code identifier is only an identifier. A multi-instance embeds an instance's state inside another FB's state; a parameter instance forwards access to existing state. Temporary variables and constants are not persistent instance fields. For a system instruction, use its documented type and exposed members; never infer private state bytes by adding only the visible input and output widths.

### 11.6 Serialization and external data exchange

A byte sequence used outside the typed PLC program needs an explicit format. At minimum, specify field order, widths, signedness, string encoding and length handling, Boolean packing, byte order, and any required alignment or reserved bytes. A remote device's register order is an additional protocol choice; it does not follow solely from the PLC's internal byte order.

The READ_BIG, WRITE_BIG, READ_LITTLE and WRITE_LITTLE instruction families allow supported scalar values to be transferred to or from ARRAY OF BYTE with an explicit byte order. Their accepted scalar types depend on the target and instruction version. A BYTE buffer can represent either order regardless of whether the buffer or source variable is optimized.

Serialize/Deserialize provide a sequential representation for supported structured data. The representation follows the documented serialization rules; it is not a dump of the source object's optimized work-memory placement. For example, later instruction versions allow an optimized ARRAY OF BYTE buffer while retaining the sequential representation used with standard buffers. Structure padding and currently unused STRING capacity may be undefined in the output. This means that a raw comparison, checksum or external protocol must account for those bytes explicitly.

Where a communication or symbolic-access instruction provides its own mode or schema, that contract takes precedence over assumptions about the source DB. The same type can have a standard-memory representation, an optimized internal representation and a protocol representation with different ordering or padding.

### 11.7 Limits of a universal byte map

An exact byte map is available only when the representation is defined. The following boundaries apply throughout this document:

| Object | What can be specified without the original compiled project |
|---|---|
| Supported elementary type in the stated standard representation | Logical encoding, nominal size, and byte/bit significance |
| Traditional POINTER or ANY | The documented descriptor map, with its form and CPU restrictions |
| Concrete standard STRUCT, UDT or fixed array | Offsets calculated from its complete declaration and the applicable compiler-era rules |
| Optimized STRUCT, UDT or fixed array | Value types and documented storage principles; not a stable universal offset map |
| VARIANT, REF_TO, ARRAY[*] parameter | Semantic behavior and declaration restrictions; no invented public internal descriptor layout |
| FB or system-instruction instance | Documented exposed fields and any published representation; the full allocation remains instance-, version- and target-dependent |
| Hardware or system identifier | Its documented base encoding; the size of the object it identifies is a different matter |

For a specific compiled application, the authoritative size calculation requires the full declaration, CPU model, configured firmware, TIA version and update, access mode, instruction versions, and relevant instance/retentivity settings.
