//! The machine pages: the list, the form, the machine page, and the unit tree
//! (`api.md` section 2.2, design D6, D7, and D12).
//!
//! The module solves one problem: each area of the plant model owns its route rows and its tests.
//! So the agents of the machine pages, the unit pages, and the type pages change disjoint files.

use axum::http::Method;
use axum::routing::{get, post};
use recipe_app::auth::Operation;
use recipe_domain::plant::PlcPlatform;

use super::path_values::query_values;
use super::texts::Texts;
use crate::routes::{RouteEntry, operation_row};

mod commands;
mod connection;
mod connection_request;
mod connection_save;
mod data_block_commands;
mod data_block_form;
mod form;
mod list;
mod page;
pub(crate) mod tree;
mod tree_page;

/// Return the route rows of the machine pages.
///
/// The rows are `/machines`, `/machines/new`, `/machines/{code}`, `/machines/by-id/{uuid}`,
/// `/machines/{code}/units`, `/machines/{code}/connection/edit`, and the commands to create,
/// rename, and archive a machine and to save its connection or its data block number.
pub(crate) fn rows() -> Vec<RouteEntry> {
    let mut rows = page_rows();
    rows.extend(command_rows());
    rows
}

/// Return the rows that show a list, a form, a machine, or its unit tree.
fn page_rows() -> Vec<RouteEntry> {
    vec![
        operation_row(
            Method::GET,
            "/machines",
            Operation::ListMachines,
            get(list::list),
        ),
        operation_row(
            Method::GET,
            "/machines/new",
            Operation::GetMachineForm,
            get(form::new_form),
        ),
        operation_row(
            Method::GET,
            "/machines/{code}",
            Operation::GetMachine,
            get(page::by_code),
        ),
        operation_row(
            Method::GET,
            "/machines/by-id/{uuid}",
            Operation::GetMachine,
            get(page::by_id),
        ),
        operation_row(
            Method::GET,
            "/machines/{code}/units",
            Operation::GetUnitTree,
            get(tree_page::show),
        ),
        operation_row(
            Method::GET,
            "/machines/{code}/connection/edit",
            Operation::GetPlcConnectionForm,
            get(connection::edit_form),
        ),
    ]
}

/// Return the rows that create, rename, archive, and change a machine.
fn command_rows() -> Vec<RouteEntry> {
    vec![
        operation_row(
            Method::POST,
            "/machines",
            Operation::CreateMachine,
            post(form::create),
        ),
        operation_row(
            Method::POST,
            "/machines/{code}/rename",
            Operation::RenameMachine,
            post(commands::rename),
        ),
        operation_row(
            Method::POST,
            "/machines/{code}/archive",
            Operation::ArchiveMachine,
            post(commands::archive),
        ),
        operation_row(
            Method::POST,
            "/machines/{code}/data-block",
            Operation::SetMachineDataBlock,
            post(data_block_commands::set),
        ),
        operation_row(
            Method::POST,
            "/machines/{code}/connection",
            Operation::SetPlcConnection,
            post(connection_save::save),
        ),
    ]
}

/// Return the key of the text of a PLC platform in the bundles.
pub(crate) const fn platform_key(platform: PlcPlatform) -> &'static str {
    match platform {
        PlcPlatform::CodesysIec => "platform-codesys-iec",
        PlcPlatform::TwinCat3 => "platform-twincat3",
        PlcPlatform::S7_1200 => "platform-s7-1200",
        PlcPlatform::S7_1500 => "platform-s7-1500",
        PlcPlatform::S7_300 => "platform-s7-300",
    }
}

/// Return the name of a PLC platform in the language of the page.
pub(crate) fn platform_text(t: Texts<'_>, platform: PlcPlatform) -> String {
    t.get(platform_key(platform))
}

#[cfg(test)]
mod commands_tests;
#[cfg(test)]
mod connection_leak_tests;
#[cfg(test)]
mod connection_lines_tests;
#[cfg(test)]
mod connection_refusal_tests;
#[cfg(test)]
mod connection_request_tests;
#[cfg(test)]
mod connection_save_tests;
#[cfg(test)]
mod connection_tests;
#[cfg(test)]
mod connection_xss_tests;
#[cfg(test)]
mod data_block_commands_tests;
#[cfg(test)]
mod form_tests;
#[cfg(test)]
mod list_tests;
#[cfg(test)]
mod machine_connection_tests;
#[cfg(test)]
mod machine_data_block_tests;
#[cfg(test)]
mod machine_tests;
#[cfg(test)]
pub(crate) mod route_tests;
#[cfg(test)]
pub(crate) mod test_world;
#[cfg(test)]
mod tree_page_tests;
#[cfg(test)]
mod tree_tests;
