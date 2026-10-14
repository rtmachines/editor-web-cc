//! The password gate and the read-only gate (design D11 step 9, D14).

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use recipe_app::auth::Operation;
use recipe_app::error::AppError;
use recipe_app::identity::Authenticated;
use recipe_domain::identity::IdentityError;

use crate::error::WebError;
use crate::routes::Access;
use crate::state::WebState;

impl Access {
    /// Return true when a user who must change the password may open the route.
    ///
    /// The routes are `GET /me/password`, `POST /me/password`, `POST /logout`, `POST /lang`, and
    /// the static files (`api.md` section 7.4).
    fn open_while_password_change(self) -> bool {
        match self {
            Self::Static | Self::Lang => true,
            Self::Operation(operation) => matches!(
                operation,
                Operation::GetOwnPasswordForm | Operation::ChangeOwnPassword | Operation::Logout
            ),
        }
    }

    /// Return true when the route still runs in read-only mode (Assumption A24).
    fn runs_in_read_only(self) -> bool {
        match self {
            Self::Static | Self::Lang => true,
            Self::Operation(operation) => operation.runs_in_read_only(),
        }
    }
}

/// Hold back a session with `must_change_password` and a command in read-only mode.
///
/// The password gate comes first and gives `E-IDN-007`, which is a `303` to `/me/password`. The
/// read-only gate gives `E-WEB-006`. A request that matched no route goes on.
pub(crate) async fn gates(
    State(state): State<WebState>,
    request: Request,
    next: Next,
) -> Result<Response, WebError> {
    let Some(access) = request.extensions().get::<Access>().copied() else {
        return Ok(next.run(request).await);
    };
    let must_change = request
        .extensions()
        .get::<Option<Authenticated>>()
        .and_then(Option::as_ref)
        .is_some_and(Authenticated::must_change_password);
    if must_change && !access.open_while_password_change() {
        return Err(WebError::App(AppError::Identity(
            IdentityError::MustChangePassword,
        )));
    }
    if state.read_only().is_on() && !access.runs_in_read_only() {
        return Err(WebError::ReadOnlyMode);
    }
    Ok(next.run(request).await)
}
