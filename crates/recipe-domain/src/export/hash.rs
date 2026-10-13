//! The content hash of the hashed body (`api.md` section 6.1, ADR-0019).

use sha2::Digest;

use super::{ExportError, HashedBody, Sha256};

impl HashedBody {
    /// Return the RFC 8785 (JCS) canonical form of the body.
    ///
    /// The keys of every object are sorted and the text has no white space. The text solves one
    /// problem: two builds of the same body give the same bytes, so they give the same hash.
    ///
    /// # Errors
    ///
    /// Returns `E-EXP-004` when the serializer fails.
    pub fn canonical_text(&self) -> Result<String, ExportError> {
        Ok(serde_json_canonicalizer::to_string(self)?)
    }

    /// Return the SHA-256 hash of the canonical form, the value of `content_sha256`.
    ///
    /// # Errors
    ///
    /// Returns `E-EXP-004` when the serializer fails.
    pub fn sha256(&self) -> Result<Sha256, ExportError> {
        let text = self.canonical_text()?;
        let digest: [u8; 32] = sha2::Sha256::digest(text.as_bytes()).into();
        Ok(Sha256::from(digest))
    }
}

#[cfg(test)]
mod tests;
