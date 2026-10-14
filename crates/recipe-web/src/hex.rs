//! Hex text of the web layer (design D12).
//!
//! The pre-session token writes and reads lower-case hex. The form decoder reads hex digits of
//! either case. This module holds the one copy of each routine.

/// Write bytes as lower-case hex.
pub(crate) fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Read one hex digit of either case.
pub(crate) fn hex_value(byte: u8) -> Option<u8> {
    char::from(byte)
        .to_digit(16)
        .and_then(|digit| u8::try_from(digit).ok())
}

/// Read `expected` bytes from lower-case hex text. Any other text gives `None`.
pub(crate) fn from_lower_hex(text: &str, expected: usize) -> Option<Vec<u8>> {
    let digits = text.as_bytes();
    if digits.len() != expected.checked_mul(2)? {
        return None;
    }
    let (pairs, _rest) = digits.as_chunks::<2>();
    pairs
        .iter()
        .map(|[high, low]| {
            lower_nibble(*high)?
                .checked_mul(16)?
                .checked_add(lower_nibble(*low)?)
        })
        .collect()
}

/// Read one lower-case hex digit. An upper-case digit gives `None`.
fn lower_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' | b'a'..=b'f' => hex_value(byte),
        _other => None,
    }
}
