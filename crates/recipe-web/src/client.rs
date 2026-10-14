//! The client address of a request (design D11 step 4, ADR-0035 p, `api.md` section 7.1).

use std::net::{IpAddr, SocketAddr};

use axum::extract::{ConnectInfo, FromRequestParts, Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;
use tracing::Span;

use crate::error::{FieldReason, WebError};
use crate::pipeline::request_id_in;
use crate::settings::CidrList;
use crate::state::WebState;

/// The name of the header that a trusted proxy sets.
const X_FORWARDED_FOR: &str = "x-forwarded-for";
/// The name of the header field in a `E-WEB-008` answer.
const FORWARDED_FIELD: &str = "X-Forwarded-For";

/// The address of the client in canonical form.
///
/// The type solves one problem: the throttle key, the audit entries, and the database must see
/// `::ffff:a.b.c.d` and `a.b.c.d` as one address. The layer stores it as a request extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClientAddr(IpAddr);

impl ClientAddr {
    /// Build the address of a test client.
    #[cfg(test)]
    pub(crate) const fn of(ip: IpAddr) -> Self {
        Self(ip)
    }

    /// Return the address.
    pub(crate) const fn ip(self) -> IpAddr {
        self.0
    }
}

/// Read every entry of every `X-Forwarded-For` header line, in order, in canonical form.
///
/// No entry is skipped. An entry that is not an address gives `E-WEB-008`.
fn forwarded_entries(headers: &HeaderMap) -> Result<Vec<IpAddr>, WebError> {
    let bad = || WebError::BadRequest {
        field: FORWARDED_FIELD,
        reason: FieldReason::Forwarded,
    };
    let mut entries = Vec::new();
    for line in headers.get_all(X_FORWARDED_FOR) {
        let text = line.to_str().map_err(|_not_text| bad())?;
        for entry in text.split(',') {
            let address: IpAddr = entry.trim().parse().map_err(|_not_address| bad())?;
            entries.push(address.to_canonical());
        }
    }
    Ok(entries)
}

/// Return the client of a request from the peer and the forwarded entries.
///
/// A peer outside the trusted proxies is the client. For a trusted peer, the right-most entry
/// that is not a trusted proxy is the client. A chain of trusted proxies only gives its first
/// entry. A trusted peer without an entry gives `None`, because the proxy address is no client.
fn client_of(
    peer: IpAddr,
    headers: &HeaderMap,
    trusted: &CidrList,
) -> Result<Option<IpAddr>, WebError> {
    let peer = peer.to_canonical();
    if !trusted.contains(peer) {
        return Ok(Some(peer));
    }
    let entries = forwarded_entries(headers)?;
    let outside = entries
        .iter()
        .rev()
        .find(|entry| !trusted.contains(**entry));
    Ok(outside.or_else(|| entries.first()).copied())
}

/// Find the client address and put a [`ClientAddr`] into the request extensions.
///
/// A request without `ConnectInfo` gives `E-WEB-010`. A trusted peer without `X-Forwarded-For`
/// also gives `E-WEB-010`. The layer never guesses an address and never logs.
pub(crate) async fn client_address(
    State(state): State<WebState>,
    request: Request,
    next: Next,
) -> Result<Response, WebError> {
    // The extractor also finds the address that `MockConnectInfo` gives a test.
    let (mut parts, body) = request.into_parts();
    let request_id = request_id_in(&parts.extensions);
    let Ok(ConnectInfo(peer)) =
        ConnectInfo::<SocketAddr>::from_request_parts(&mut parts, &state).await
    else {
        return Err(WebError::Internal { request_id });
    };
    let Some(client) = client_of(peer.ip(), &parts.headers, &state.settings().trusted_proxies)?
    else {
        return Err(WebError::Internal { request_id });
    };
    let address = ClientAddr(client);
    Span::current().record("client_ip", tracing::field::display(address.ip()));
    parts.extensions.insert(address);
    Ok(next.run(Request::from_parts(parts, body)).await)
}
