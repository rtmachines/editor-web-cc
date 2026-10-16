//! The approval pages: the preview `GET /recipes/{ref}/versions/{n}/approve` and the approval
//! `POST /recipes/{ref}/versions/{n}/approve` (ADR-0049, design D3 and D7).

pub(crate) mod approve;
mod blockers;
pub(crate) mod preview;

#[cfg(test)]
mod approve_error_tests;
#[cfg(test)]
mod approve_tests;
#[cfg(test)]
mod blockers_tests;
#[cfg(test)]
mod preview_outdated_tests;
#[cfg(test)]
mod preview_page_tests;
#[cfg(test)]
mod preview_rows_tests;
#[cfg(test)]
mod preview_state_tests;
#[cfg(test)]
mod preview_tests;
#[cfg(test)]
mod two_session_tests;
