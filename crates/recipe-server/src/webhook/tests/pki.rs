//! Fixed test certificates for the webhook tests (design D14 row T27).
//!
//! The keys below protect nothing. They exist only for tests on `127.0.0.1`. They were made with
//! OpenSSL 3.5 on 2026-10-25, with EC P-256 keys and a validity of 100 years (until 2126-10-01):
//!
//! 1. `openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes -days 36500`
//!    makes the self-signed authorities A and B (`basicConstraints=CA:TRUE`).
//! 2. `openssl genpkey -algorithm EC` and `openssl req -new` make the key and the request of the
//!    leaf.
//! 3. `openssl x509 -req -CA <A> -days 36500` signs the leaf. The leaf has the alternative names
//!    `IP:127.0.0.1` and `DNS:localhost`, the usage `serverAuth`, and `CA:FALSE`.

/// The authority A. It signed [`LEAF_CERTIFICATE`].
pub(super) const CA_A: &str = "-----BEGIN CERTIFICATE-----
MIIBqjCCAVGgAwIBAgIUW75TdZ3NGsgVBvWcifKDHo9cUy8wCgYIKoZIzj0EAwIw
IjEgMB4GA1UEAwwXUmVjaXBlIGVkaXRvciB0ZXN0IENBIEEwIBcNMjYxMDI1MDcz
MDEyWhgPMjEyNjEwMDEwNzMwMTJaMCIxIDAeBgNVBAMMF1JlY2lwZSBlZGl0b3Ig
dGVzdCBDQSBBMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE0qAeXKHGBg6SaBol
SREuFeSPnl1J7Z1ZpiznFxtYoCkJhmJN4ZGPyuC33o1ytAmn2MLtaik3NTLh2llz
7F8ilKNjMGEwHQYDVR0OBBYEFKryriKwrTfG/YpmGIaj+hkhd+X+MB8GA1UdIwQY
MBaAFKryriKwrTfG/YpmGIaj+hkhd+X+MA8GA1UdEwEB/wQFMAMBAf8wDgYDVR0P
AQH/BAQDAgEGMAoGCCqGSM49BAMCA0cAMEQCIBv4oCyAHZw++t4Qn5Rx2ZlqTQH7
hDYss8b+YjyKsVg7AiBZI2R3ZUv2aQerD8UDzn2d1Q+kd7WhSUZoJ2aQJTNJCg==
-----END CERTIFICATE-----
";

/// The authority B. It signed nothing that the tests use. It is an unrelated authority.
pub(super) const CA_B: &str = "-----BEGIN CERTIFICATE-----
MIIBrDCCAVGgAwIBAgIUZSfqIWCVZFBn3iQbpZfgXzN5EsAwCgYIKoZIzj0EAwIw
IjEgMB4GA1UEAwwXUmVjaXBlIGVkaXRvciB0ZXN0IENBIEIwIBcNMjYxMDI1MDcz
MDEyWhgPMjEyNjEwMDEwNzMwMTJaMCIxIDAeBgNVBAMMF1JlY2lwZSBlZGl0b3Ig
dGVzdCBDQSBCMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEoQ/PgB0TZb3FG5N1
k7mZx2gOODJFQh6lzvAQVhaLniUDQsjDVrryylCeUEuSioEdm5FnUjc0v1cIrc7O
eUKy1KNjMGEwHQYDVR0OBBYEFPE/5KfrtskhicCp1xOmLQgdAWaDMB8GA1UdIwQY
MBaAFPE/5KfrtskhicCp1xOmLQgdAWaDMA8GA1UdEwEB/wQFMAMBAf8wDgYDVR0P
AQH/BAQDAgEGMAoGCCqGSM49BAMCA0kAMEYCIQCdhStZiKxuJ9oyKi2o5184tI75
5Cigzbv8YwRQfPgINgIhANmxNN+1hXoa1CJsGN/p819449YJ4fTscF4lAskMb247
-----END CERTIFICATE-----
";

/// The certificate of the test server for `127.0.0.1`. The authority A signed it.
pub(super) const LEAF_CERTIFICATE: &str = "-----BEGIN CERTIFICATE-----
MIIBzTCCAXOgAwIBAgIUBBXi4kq7HBAqlCZAhnGgAh6f/HEwCgYIKoZIzj0EAwIw
IjEgMB4GA1UEAwwXUmVjaXBlIGVkaXRvciB0ZXN0IENBIEEwIBcNMjYxMDI1MDcz
MDEzWhgPMjEyNjEwMDEwNzMwMTNaMBQxEjAQBgNVBAMMCTEyNy4wLjAuMTBZMBMG
ByqGSM49AgEGCCqGSM49AwEHA0IABJ9kdQm7lIcXNSWCCK2BxeqEyio8eudbz+ez
ZB+1ICYyu0hl0bTnBZ9MwN3t9N0cJs6UIXzUwhaX0THTamvRntqjgZIwgY8wGgYD
VR0RBBMwEYcEfwAAAYIJbG9jYWxob3N0MBMGA1UdJQQMMAoGCCsGAQUFBwMBMAwG
A1UdEwEB/wQCMAAwDgYDVR0PAQH/BAQDAgeAMB0GA1UdDgQWBBRSqam2ZpNUF9Nj
z6T6nJj2bk5WQDAfBgNVHSMEGDAWgBSq8q4isK03xv2KZhiGo/oZIXfl/jAKBggq
hkjOPQQDAgNIADBFAiEAs7S2Ckc9z9krMuOQYK50a7Poswyd8ucFhsFkv6ZsOoIC
IH7OdOwFvXfOe8v/mew+9irp9BGcHHuC5EVO6rSBnNBG
-----END CERTIFICATE-----
";

/// The private key of [`LEAF_CERTIFICATE`] in PKCS #8 form. It is a test key.
pub(super) const LEAF_KEY: &str = "-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgPZhKfoYluvdQ1YXr
7eBBBofVop0HCI1LIGTLiLtp8yKhRANCAASfZHUJu5SHFzUlggitgcXqhMoqPHrn
W8/ns2QftSAmMrtIZdG05wWfTMDd7fTdHCbOlCF81MIWl9Ex02pr0Z7a
-----END PRIVATE KEY-----
";
