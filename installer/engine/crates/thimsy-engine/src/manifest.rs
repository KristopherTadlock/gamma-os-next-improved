//! Signed catalog manifests (minisign). A downloaded catalog is only used when
//! its signature verifies against a key built into the app.

use anyhow::{bail, Context, Result};
use minisign_verify::{PublicKey, Signature};

/// Public keys the app trusts for catalog updates (current, next).
/// Empty in test builds: remote catalog updates are disabled until a release
/// key exists; the bundled catalog ships inside the app.
pub const TRUSTED_KEYS: &[&str] = &[];

/// Verifies `data` against a minisign `signature` with any of `keys`.
pub fn verify(data: &[u8], signature: &str, keys: &[&str]) -> Result<()> {
    if keys.is_empty() {
        bail!("no trusted catalog keys in this build");
    }
    let sig = Signature::decode(signature).context("bad signature format")?;
    for k in keys {
        let pk = PublicKey::from_base64(k).context("bad public key")?;
        if pk.verify(data, &sig, false).is_ok() {
            return Ok(());
        }
    }
    bail!("catalog signature does not verify")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test vector from the minisign-verify crate's documentation (prehashed).
    const PK: &str = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
    const SIG: &str = "untrusted comment: signature from minisign secret key
RUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=
trusted comment: timestamp:1633700835\tfile:test\tprehashed
wLMDjy9FLAuxZ3q4NlEvkgtyhrr0gtTu6KC4KBJdITbbOeAi1zBIYo0v4iTgt8jJpIidRJnp94ABQkJAgAooBQ==";

    #[test]
    fn verifies_known_signature() {
        verify(b"test", SIG, &[PK]).unwrap();
        assert!(verify(b"tampered", SIG, &[PK]).is_err());
        assert!(verify(b"test", SIG, &[]).is_err());
    }
}
