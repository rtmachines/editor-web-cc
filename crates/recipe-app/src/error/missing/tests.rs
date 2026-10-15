use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{MachineCode, TypeName};

use super::Missing;
use crate::error::AppError;

fn machine_code() -> MachineCode {
    MachineCode::parse("M12").unwrap()
}

fn type_name() -> TypeName {
    TypeName::parse("ST_Zone").unwrap()
}

#[test]
fn missing_malformed_machine_names_a_machine_and_never_the_typed_text() {
    let text = Missing::MalformedMachine.to_string();
    assert_eq!(text, "Machine");
    let error = AppError::NotFound {
        entity: Missing::MalformedMachine,
    };
    assert_eq!(error.code(), ErrorCode::WEB_005);
}

#[test]
fn missing_malformed_type_names_a_type_and_never_the_typed_text() {
    let text = Missing::MalformedType.to_string();
    assert_eq!(text, "Type");
    let error = AppError::NotFound {
        entity: Missing::MalformedType,
    };
    assert_eq!(error.code(), ErrorCode::WEB_005);
}

#[test]
fn missing_type_revision_names_the_revision_the_type_and_the_machine() {
    let missing = Missing::TypeRevision {
        code: machine_code(),
        name: type_name(),
        revision: 9,
    };
    assert_eq!(
        missing.to_string(),
        "Revision 9 of type ST_Zone of machine M12"
    );
    let error = AppError::NotFound { entity: missing };
    assert_eq!(error.code(), ErrorCode::WEB_005);
}
