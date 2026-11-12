//! Binary deltas (bsdiff 4 format) between a file in the user's own firmware
//! and the Thimsy version of it, checked by input and output hash.

use crate::error::{Code, EngineError};
use crate::util::sha256_hex;
use anyhow::{Context, Result};

/// Applies `patch` to `source`. Both hashes are checked when given.
pub fn apply(
    source: &[u8],
    patch: &[u8],
    in_sha256: Option<&str>,
    out_sha256: Option<&str>,
    what: &str,
) -> Result<Vec<u8>> {
    if let Some(want) = in_sha256 {
        let got = sha256_hex(source);
        if got != want {
            return Err(EngineError::new(
                Code::HashMismatch,
                format!("{what}: the firmware's copy isn't the one this delta was made for (sha256 {got}, expected {want})"),
            )
            .hint("Use the firmware version the catalog lists for this device.")
            .into());
        }
    }
    let p = qbsdiff::Bspatch::new(patch).with_context(|| format!("{what}: bad delta"))?;
    let mut out = Vec::with_capacity(p.hint_target_size() as usize);
    p.apply(source, &mut out)
        .with_context(|| format!("{what}: applying delta"))?;
    if let Some(want) = out_sha256 {
        let got = sha256_hex(&out);
        if got != want {
            return Err(EngineError::new(
                Code::HashMismatch,
                format!("{what}: patched result has sha256 {got}, expected {want}"),
            )
            .into());
        }
    }
    Ok(out)
}

/// Makes a delta from `source` to `target` (maintainer tooling).
pub fn make(source: &[u8], target: &[u8]) -> Result<Vec<u8>> {
    let mut patch = Vec::new();
    qbsdiff::Bsdiff::new(source, target)
        .compression_level(9)
        .compare(std::io::Cursor::new(&mut patch))?;
    Ok(patch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_hash_checks() {
        let a: Vec<u8> = (0..100_000u32).map(|i| (i % 200) as u8).collect();
        let mut b = a.clone();
        b[500..600].fill(7);
        b.extend_from_slice(b"thimsy");
        let p = make(&a, &b).unwrap();
        assert!(p.len() < 2000);
        let out = apply(&a, &p, Some(&sha256_hex(&a)), Some(&sha256_hex(&b)), "t").unwrap();
        assert_eq!(out, b);
        let err = apply(&b, &p, Some(&sha256_hex(&a)), None, "t").unwrap_err();
        assert_eq!(crate::error::code_of(&err).0, Code::HashMismatch);
    }
}
