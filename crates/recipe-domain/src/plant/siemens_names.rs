//! Siemens reserved struct names on the S7 platforms (ADR-0186).
//!
//! A TIA project cannot hold a PLC data type with the name of a Siemens type or
//! keyword. A struct of the editor with such a name can never match a real UDT.

use super::{DerivedTypeReason, PlantError, PlcPlatform, TypeName};

/// Prefix of the telegram family of ADR-0186 (d3), in upper case.
const TELEGRAM_PREFIX: &str = "PD_TEL";

/// The 253 Siemens type names of ADR-0186 (d1) and the 16 Siemens SCL keywords of
/// ADR-0186 (d2). A struct name on an S7 platform is none of them, in any case.
///
/// The family `PD_TEL<n>` of ADR-0186 (d3) is not in the list. [`is_siemens_reserved_name`]
/// holds it as a pattern. The ops query `deploy/db/check-identifier-rules.sql` holds
/// a second copy, and a test compares both copies.
pub const SIEMENS_RESERVED_NAMES: [&str; 269] = [
    "AlarmData",
    "AOM_IDENT",
    "AssocValues",
    "AXIS_REF",
    "BLOCK_DB",
    "BLOCK_FB",
    "BLOCK_FC",
    "BLOCK_SDB",
    "CONDITIONS",
    "Conf_ClientId",
    "Conf_ClientId_Opaque",
    "Conf_DNS",
    "Conf_DNS_IPV4",
    "Conf_Domainname",
    "Conf_Hostname",
    "Conf_IPSuitev4",
    "Conf_IPSuitev4_IPV4",
    "Conf_NTP",
    "CONN_ANY",
    "CONN_OUC",
    "CONN_PRG",
    "CONN_R_ID",
    "CONT_C",
    "CONT_S",
    "COUNTER",
    "CREF",
    "CTD_DINT",
    "CTD_INT",
    "CTD_LINT",
    "CTD_SINT",
    "CTD_UDINT",
    "CTD_UINT",
    "CTD_ULINT",
    "CTD_USINT",
    "CTUD_DINT",
    "CTUD_INT",
    "CTUD_LINT",
    "CTUD_SINT",
    "CTUD_UDINT",
    "CTUD_UINT",
    "CTUD_ULINT",
    "CTUD_USINT",
    "CTU_DINT",
    "CTU_INT",
    "CTU_LINT",
    "CTU_SINT",
    "CTU_UDINT",
    "CTU_UINT",
    "CTU_ULINT",
    "CTU_USINT",
    "C_ALARM",
    "C_ALARM_8",
    "C_ALARM_8P",
    "C_ALARM_S",
    "C_ALARM_SD",
    "C_AR_SEND",
    "C_NOTIFY",
    "C_NOTIFY_8P",
    "DATE_AND_LTIME",
    "DB_ANY",
    "DB_DYN",
    "DB_WWW",
    "DIS",
    "DNN",
    "DX_TEL_SyncOp",
    "ERROR_STRUCT",
    "EVENT_ANY",
    "EVENT_ATT",
    "EVENT_HWINT",
    "Filter_DT1",
    "Filter_PT1",
    "Filter_PT2",
    "Filter_Universal",
    "GEOADDR",
    "High_Speed_Counter",
    "HSC_Count",
    "HSC_Frequency",
    "HSC_Period",
    "HW_ANY",
    "HW_DEVICE",
    "HW_DPMASTER",
    "HW_DPSLAVE",
    "HW_HSC",
    "HW_IEPORT",
    "HW_INTERFACE",
    "HW_IO",
    "HW_IOSYSTEM",
    "HW_MODULE",
    "HW_PTO",
    "HW_PWM",
    "HW_SUBMODULE",
    "IEC_COUNTER",
    "IEC_DCOUNTER",
    "IEC_LCOUNTER",
    "IEC_LTIMER",
    "IEC_SCOUNTER",
    "IEC_TIMER",
    "IEC_UCOUNTER",
    "IEC_UDCOUNTER",
    "IEC_ULCOUNTER",
    "IEC_USCOUNTER",
    "IF_CONF_DNS",
    "IF_CONF_HEADER",
    "IF_CONF_MAC",
    "IF_CONF_NOS",
    "IF_CONF_NTP",
    "IF_CONF_V4",
    "IF_CONF_V6",
    "IM0_Data",
    "INSTANCE",
    "IP_V4",
    "IP_V6",
    "NREF",
    "OB_ANY",
    "OB_ATT",
    "OB_CYCLIC",
    "OB_DELAY",
    "OB_DIAG",
    "OB_HWINT",
    "OB_PCYCLE",
    "OB_STARTUP",
    "OB_TIMEERROR",
    "OB_TOD",
    "OPC_UA_BrowsePath",
    "OPC_UA_ByteStringActualLength",
    "OPC_UA_LocalizedTextEncodingMask",
    "OPC_UA_NodeAdditionalInfo",
    "OPC_UA_NodeAdditionalInfoExt",
    "OPC_UA_NodeId",
    "OPC_UA_QualifiedName",
    "OPC_UA_RelativePath",
    "OPC_UA_RelativePathElement",
    "OPC_UA_SessionConnectInfo",
    "PARAMETER",
    "PD_STW1_611Umode",
    "PD_STW2_611Umode",
    "PD_ZSW1_611Umode",
    "PD_ZSW2_611Umode",
    "PID_3Step",
    "PID_Compact",
    "PID_CP",
    "PID_ES",
    "PID_Temp",
    "PIP",
    "Polyline",
    "PORT",
    "PULSEGEN",
    "RampFunction",
    "RampSoak",
    "REMOTE",
    "ResolvedSymbol",
    "RTM",
    "SI_AccessError",
    "SI_classic",
    "SI_CPURedundancyError",
    "SI_CPURedundancyError_V2",
    "SI_Cyclic",
    "SI_Delay",
    "SI_DiagnosticInterrupt",
    "SI_HWInterrupt",
    "SI_HWInterrupt_Extended",
    "SI_IORedundancyError",
    "SI_IORedundancyError_V2",
    "SI_Ipo",
    "SI_none",
    "SI_PlugPullModule",
    "SI_PreInterpolator",
    "SI_PreOrPostServo",
    "SI_ProgIOAccessError",
    "SI_ProgramCycle",
    "SI_Servo",
    "SI_Startup",
    "SI_StationFailure",
    "SI_Submodule",
    "SI_SynchCycle",
    "SI_TimeError",
    "SI_TimeOfDay",
    "SplitRange",
    "SSI_Absolute_Encoder",
    "SSL_HEADER",
    "TADDR_Param",
    "TADDR_RCV_IP",
    "TADDR_SEND_QDN",
    "TCONT_CP",
    "TCONT_S",
    "TCON_Configured",
    "TCON_DNS",
    "TCON_FDL",
    "TCON_IPMulticastTTL",
    "TCON_IP_RFC",
    "TCON_IP_v4",
    "TCON_IP_V4_SEC",
    "TCON_ISOnative",
    "TCON_Param",
    "TCON_PHONE",
    "TCON_QDN",
    "TCON_QDN_SEC",
    "TCON_TCPTermination",
    "TIMER",
    "TimeTransformationRule",
    "TMail_FileReference",
    "TMAIL_FQDN",
    "TMAIL_QDN_SEC",
    "TMAIL_V4",
    "TMAIL_V4_SEC",
    "TMAIL_V6",
    "TMAIL_V6_SEC",
    "TOF_LTIME",
    "TOF_TIME",
    "TONR_LTIME",
    "TONR_TIME",
    "TON_LTIME",
    "TON_TIME",
    "TO_Axis",
    "TO_BasicPos",
    "TO_Cam",
    "TO_CamBase",
    "TO_CamTrack",
    "TO_Cam_10k",
    "TO_Cam_600Seg",
    "TO_Cam_6kSeg",
    "TO_CommandTable",
    "TO_ExternalEncoder",
    "TO_IDENT",
    "TO_Interpreter",
    "TO_InterpreterMapping",
    "TO_InterpreterProgram",
    "TO_Kinematics",
    "TO_LeadingAxisProxy",
    "TO_MeasuringInput",
    "TO_Object",
    "TO_OutputCam",
    "TO_PositioningAxis",
    "TO_SpeedAxis",
    "TO_Struct_Ipr_AxPosition",
    "TO_Struct_Ipr_Frame",
    "TO_Struct_Ipr_JtPosition",
    "TO_Struct_Ipr_Position",
    "TO_Struct_Kinematics_Frame",
    "TO_Struct_Kinematics_KinematicsFrame",
    "TO_Struct_TransformationParameter_V1",
    "TO_Struct_TransformationParameter_V2",
    "TO_SynchronousAxis",
    "TO_TagFields",
    "TO_TAGLAYOUT",
    "TP_LTIME",
    "TP_TIME",
    "TSelector",
    "TUN_EC",
    "TUN_ES",
    "VARIANT",
    "VOID",
    "VREF",
    "BEGIN",
    "BLOCK_SFB",
    "BLOCK_SFC",
    "CONST",
    "DATA_BLOCK",
    "END_CONST",
    "END_DATA_BLOCK",
    "END_LABEL",
    "END_ORGANIZATION_BLOCK",
    "END_REGION",
    "GOTO",
    "LABEL",
    "NIL",
    "OK",
    "ORGANIZATION_BLOCK",
    "REGION",
];

/// Tell whether a text is the telegram family `PD_TEL<n>` of ADR-0186 (d3), in any case.
///
/// The family is `PD_TEL` and one or more ASCII digits. A leading zero is a digit.
/// The family is a deliberate superset of the telegram names of the Siemens source.
fn is_telegram_name(text: &str) -> bool {
    let Some(prefix) = text.get(..TELEGRAM_PREFIX.len()) else {
        return false;
    };
    let Some(digits) = text.get(TELEGRAM_PREFIX.len()..) else {
        return false;
    };
    prefix.eq_ignore_ascii_case(TELEGRAM_PREFIX)
        && !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
}

/// Tell whether a text is a Siemens reserved name, in any case (ADR-0186 (d)).
///
/// The text is a name of [`SIEMENS_RESERVED_NAMES`] or matches the family `PD_TEL<n>`.
/// The function does not read the platform. [`check_siemens_struct_name`] does.
#[must_use]
pub fn is_siemens_reserved_name(text: &str) -> bool {
    SIEMENS_RESERVED_NAMES
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(text))
        || is_telegram_name(text)
}

/// Refuse a struct name that is a Siemens reserved name on an S7 platform (ADR-0186 (e), (f)).
///
/// This check solves the problem that a form parses a name before it knows the platform
/// (ADR-0186 (g)). The use case calls it with the platform of the machine.
/// `CodesysIec` and `TwinCat3` accept every name.
///
/// # Errors
///
/// Returns [`PlantError::DerivedTypeInvalid`] with [`DerivedTypeReason::ReservedName`]
/// (`E-PLT-013`) when the platform is an S7 platform and the name is a Siemens reserved name.
pub fn check_siemens_struct_name(platform: PlcPlatform, name: &TypeName) -> Result<(), PlantError> {
    if platform.is_s7() && is_siemens_reserved_name(name.as_str()) {
        return Err(PlantError::DerivedTypeInvalid {
            type_expr: name.to_string(),
            reason: DerivedTypeReason::ReservedName,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
