# ADR-0186: Siemens type names are reserved struct names on S7

Status: Accepted. Amends ADR-0143 (c), (e), (i2), and (i3) (reserved struct names on the S7 platforms) and ADR-0120 (a) (earlier rule set)
Date: 2026-11-17
Deciders: project owner

## Context

The Siemens type coverage review of 2026-11-17 asked OQ11 and found R04 (`docs/plan/review/2026-11-17-siemens-type-coverage-review.md` §3 and §4).

These facts shape the decision:

- ADR-0143 (c) reserves the IEC and TwinCAT keywords and the 29 primitive names as struct names. It does not hold the Siemens names.
- ADR-0143 records the risk that Siemens SCL keywords such as `BEGIN` or `DATA_BLOCK` are missing. Its mitigation is vendor point R4: a new ADR adds them.
- A user can define a struct `DATE_AND_LTIME`, `IEC_TIMER`, or `TIMER` on an S7 machine. A TIA project cannot hold a PLC data type with the name of a Siemens type. Such a struct can never match a real UDT.
- ADR-0185 names the Siemens type families outside the type matrix.
- A form of a new struct parses the name before the use case reads the platform of the machine.

The project owner decided on 2026-11-17: reserve the Siemens names on the S7 platforms, with the earlier rule set for stored rows.

## Decision drivers

- A struct of the editor can match a UDT of the PLC (R03, R13).
- The domain checks all external input (R38).
- A rule change does not lock a draft, and a stored type stays readable (ADR-0120, R19).
- One list lives in one place, and a test compares each copy.

## Considered options

### Option A: Reserve the Siemens names on the S7 platforms only

- Pros: `CodesysIec` and `TwinCat3` keep their names. The rule follows the PLC of the machine.
- Cons: the rule needs the platform, so it runs in the use case and not in the form parser.

### Option B: Reserve the Siemens names on every platform

- Pros: one list for each struct name, and the form parser can refuse the name.
- Cons: a CODESYS plant loses names such as `PORT` or `Filter_PT1` without a reason.

## Decision

The project uses **Option A**.

- (a) On `S7_300`, `S7_1200`, and `S7_1500`, a struct name is not a Siemens reserved name, in any case. The list of item (d) holds the Siemens reserved names. This item amends ADR-0143 (c).
- (b) The reserved words of ADR-0143 (c) and (e) stay reserved on every platform.
- (c) A member name can be a Siemens reserved name (ADR-0143 (l)).
- (d) The list has 269 names and one family of names. `recipe-domain` holds the one copy. The ops query of item (k) holds a second copy, and a test compares both copies. This item amends ADR-0143 (e).
  - (d1) The 253 Siemens type names come from the inventory of the review (§2.1). They are the names of the rows outside the type matrix, and the alias `DATE_AND_LTIME` of `LDT`. The families `CTU_*`, `CTD_*`, and `CTUD_*` hold the eight integer types `SINT` to `ULINT`.
  - (d2) The 16 Siemens SCL keywords are the words of the S7-SCL keyword list that ADR-0143 (d) does not hold.
  - (d3) The family `PD_TEL<n>` holds the process-data telegram structures. `<n>` is a decimal telegram number of one or more digits. An example is `PD_TEL3` (`docs/plan/review/sources/Siemens_PLC_Data_Types_Detailed_EN.md:1586` and `:1595`). The text `PD_TELx` is a naming pattern and not a type name, so the list does not hold it. The family is a deliberate superset. The source names a template and gives no list of telegram numbers (`:1586`, `:1595`). Thus the family can only block more names, never fewer. A leading zero counts as a digit, so `PD_TEL03` and `PD_TEL0` are in the family. The name `PD_TEL3_IN` is not in the family (`:1595`).
- (e) Codes. A new struct or a new revision of `DefineUdt` on an S7 machine can have a Siemens reserved name. Then `DefineUdt` gives `E-PLT-013` with the reason "reserved name" (ADR-0143 (f), (i3)). This item amends ADR-0143 (i3).
- (f) A command on an S7 machine can write a type text that names a stored struct with a Siemens reserved name. Then the command gives `E-PLT-013` with the reason "reserved name". The texts are those of ADR-0143 (i2). This item amends ADR-0143 (i2).
- (g) The form of a new struct does not refuse a Siemens reserved name, because it does not know the platform. The use case refuses it with item (e).
- (h) The parameter form and the struct form of an S7 machine propose types. The proposals leave out a stored struct with a Siemens reserved name (ADR-0143 (i2)).
- (i) Stored rows. A stored struct with a Siemens reserved name stays readable. A stored parameter type keeps it while a command does not change the type. `RebindTypeRevision` keeps it. The earlier rule set accepts such a stored name. This item amends ADR-0120 (a).
- (j) The rule has no branch of `RuleSet`. `TypeName::from_stored` and `PlcType::from_stored` do not read the platform, so a command runs the rule on the data that it writes (ADR-0166 (t)).
- (k) The ops query `deploy/db/check-identifier-rules.sql` lists each stored struct on an S7 machine with a Siemens reserved name, in a new part `s7_reserved_struct_name`. The query holds the family of item (d3) as one pattern.

The Siemens type names of item (d1):

```text
AlarmData
AOM_IDENT
AssocValues
AXIS_REF
BLOCK_DB
BLOCK_FB
BLOCK_FC
BLOCK_SDB
CONDITIONS
Conf_ClientId
Conf_ClientId_Opaque
Conf_DNS
Conf_DNS_IPV4
Conf_Domainname
Conf_Hostname
Conf_IPSuitev4
Conf_IPSuitev4_IPV4
Conf_NTP
CONN_ANY
CONN_OUC
CONN_PRG
CONN_R_ID
CONT_C
CONT_S
COUNTER
CREF
CTD_DINT
CTD_INT
CTD_LINT
CTD_SINT
CTD_UDINT
CTD_UINT
CTD_ULINT
CTD_USINT
CTUD_DINT
CTUD_INT
CTUD_LINT
CTUD_SINT
CTUD_UDINT
CTUD_UINT
CTUD_ULINT
CTUD_USINT
CTU_DINT
CTU_INT
CTU_LINT
CTU_SINT
CTU_UDINT
CTU_UINT
CTU_ULINT
CTU_USINT
C_ALARM
C_ALARM_8
C_ALARM_8P
C_ALARM_S
C_ALARM_SD
C_AR_SEND
C_NOTIFY
C_NOTIFY_8P
DATE_AND_LTIME
DB_ANY
DB_DYN
DB_WWW
DIS
DNN
DX_TEL_SyncOp
ERROR_STRUCT
EVENT_ANY
EVENT_ATT
EVENT_HWINT
Filter_DT1
Filter_PT1
Filter_PT2
Filter_Universal
GEOADDR
High_Speed_Counter
HSC_Count
HSC_Frequency
HSC_Period
HW_ANY
HW_DEVICE
HW_DPMASTER
HW_DPSLAVE
HW_HSC
HW_IEPORT
HW_INTERFACE
HW_IO
HW_IOSYSTEM
HW_MODULE
HW_PTO
HW_PWM
HW_SUBMODULE
IEC_COUNTER
IEC_DCOUNTER
IEC_LCOUNTER
IEC_LTIMER
IEC_SCOUNTER
IEC_TIMER
IEC_UCOUNTER
IEC_UDCOUNTER
IEC_ULCOUNTER
IEC_USCOUNTER
IF_CONF_DNS
IF_CONF_HEADER
IF_CONF_MAC
IF_CONF_NOS
IF_CONF_NTP
IF_CONF_V4
IF_CONF_V6
IM0_Data
INSTANCE
IP_V4
IP_V6
NREF
OB_ANY
OB_ATT
OB_CYCLIC
OB_DELAY
OB_DIAG
OB_HWINT
OB_PCYCLE
OB_STARTUP
OB_TIMEERROR
OB_TOD
OPC_UA_BrowsePath
OPC_UA_ByteStringActualLength
OPC_UA_LocalizedTextEncodingMask
OPC_UA_NodeAdditionalInfo
OPC_UA_NodeAdditionalInfoExt
OPC_UA_NodeId
OPC_UA_QualifiedName
OPC_UA_RelativePath
OPC_UA_RelativePathElement
OPC_UA_SessionConnectInfo
PARAMETER
PD_STW1_611Umode
PD_STW2_611Umode
PD_ZSW1_611Umode
PD_ZSW2_611Umode
PID_3Step
PID_Compact
PID_CP
PID_ES
PID_Temp
PIP
Polyline
PORT
PULSEGEN
RampFunction
RampSoak
REMOTE
ResolvedSymbol
RTM
SI_AccessError
SI_classic
SI_CPURedundancyError
SI_CPURedundancyError_V2
SI_Cyclic
SI_Delay
SI_DiagnosticInterrupt
SI_HWInterrupt
SI_HWInterrupt_Extended
SI_IORedundancyError
SI_IORedundancyError_V2
SI_Ipo
SI_none
SI_PlugPullModule
SI_PreInterpolator
SI_PreOrPostServo
SI_ProgIOAccessError
SI_ProgramCycle
SI_Servo
SI_Startup
SI_StationFailure
SI_Submodule
SI_SynchCycle
SI_TimeError
SI_TimeOfDay
SplitRange
SSI_Absolute_Encoder
SSL_HEADER
TADDR_Param
TADDR_RCV_IP
TADDR_SEND_QDN
TCONT_CP
TCONT_S
TCON_Configured
TCON_DNS
TCON_FDL
TCON_IPMulticastTTL
TCON_IP_RFC
TCON_IP_v4
TCON_IP_V4_SEC
TCON_ISOnative
TCON_Param
TCON_PHONE
TCON_QDN
TCON_QDN_SEC
TCON_TCPTermination
TIMER
TimeTransformationRule
TMail_FileReference
TMAIL_FQDN
TMAIL_QDN_SEC
TMAIL_V4
TMAIL_V4_SEC
TMAIL_V6
TMAIL_V6_SEC
TOF_LTIME
TOF_TIME
TONR_LTIME
TONR_TIME
TON_LTIME
TON_TIME
TO_Axis
TO_BasicPos
TO_Cam
TO_CamBase
TO_CamTrack
TO_Cam_10k
TO_Cam_600Seg
TO_Cam_6kSeg
TO_CommandTable
TO_ExternalEncoder
TO_IDENT
TO_Interpreter
TO_InterpreterMapping
TO_InterpreterProgram
TO_Kinematics
TO_LeadingAxisProxy
TO_MeasuringInput
TO_Object
TO_OutputCam
TO_PositioningAxis
TO_SpeedAxis
TO_Struct_Ipr_AxPosition
TO_Struct_Ipr_Frame
TO_Struct_Ipr_JtPosition
TO_Struct_Ipr_Position
TO_Struct_Kinematics_Frame
TO_Struct_Kinematics_KinematicsFrame
TO_Struct_TransformationParameter_V1
TO_Struct_TransformationParameter_V2
TO_SynchronousAxis
TO_TagFields
TO_TAGLAYOUT
TP_LTIME
TP_TIME
TSelector
TUN_EC
TUN_ES
VARIANT
VOID
VREF
```

The Siemens SCL keywords of item (d2):

```text
BEGIN
BLOCK_SFB
BLOCK_SFC
CONST
DATA_BLOCK
END_CONST
END_DATA_BLOCK
END_LABEL
END_ORGANIZATION_BLOCK
END_REGION
GOTO
LABEL
NIL
OK
ORGANIZATION_BLOCK
REGION
```

## Consequences

### Positive

- A new struct on an S7 machine never has the name of a Siemens type or keyword.
- `CodesysIec` and `TwinCat3` keep every name that they accept now.

### Negative

- The rule runs in the use case and not in the form parser. A new struct with a Siemens name gets `E-PLT-013` and not a field line.
- A new TIA version can add a type name that the list does not hold.

### Risks and mitigations

- Risk: the keyword list of item (d2) misses a word of TIA Portal V21. Mitigation: `design.md` of the change `siemens-type-rules` holds the open question. A new ADR adds a word.
- Risk: the stored data holds a struct with a Siemens name. Mitigation: the ops query of item (k) lists it before an upgrade.

## Related

- ADR-0120 (earlier rule set), ADR-0143 (struct names, vendor point R4), ADR-0166 (rules without a branch), ADR-0185 (excluded Siemens families)
- `domain-model.md` §2.1 (`TypeName`), `reliability.md` §1.2 (`E-PLT-013`)
- `overview.md` §7: R03, R13, R19, R38
