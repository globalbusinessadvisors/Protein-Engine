use crate::error::LedgerError;
use crate::types::MlDsaSignature;

/// Trait for cryptographic signing and verification.
///
/// In production, backed by ML-DSA (FIPS 204). Mockable for London School TDD.
#[cfg_attr(test, mockall::automock)]
pub trait CryptoSigner: Send + Sync {
    fn sign(&self, data: &[u8]) -> Result<MlDsaSignature, LedgerError>;
    fn verify(&self, data: &[u8], sig: &MlDsaSignature) -> Result<bool, LedgerError>;
}

/// Production ML-DSA signer backed by pqcrypto-mldsa (native only).
#[cfg(feature = "native")]
pub struct MlDsaSigner {
    signing_key: pqcrypto_mldsa::mldsa65::SecretKey,
    verification_key: pqcrypto_mldsa::mldsa65::PublicKey,
}

#[cfg(feature = "native")]
impl MlDsaSigner {
    /// Generate a fresh ML-DSA-65 keypair.
    pub fn generate() -> Self {
        let (pk, sk) = pqcrypto_mldsa::mldsa65::keypair();
        Self {
            signing_key: sk,
            verification_key: pk,
        }
    }

    /// Construct from an existing keypair.
    pub fn from_keys(
        verification_key: pqcrypto_mldsa::mldsa65::PublicKey,
        signing_key: pqcrypto_mldsa::mldsa65::SecretKey,
    ) -> Self {
        Self {
            signing_key,
            verification_key,
        }
    }
}

#[cfg(feature = "native")]
impl CryptoSigner for MlDsaSigner {
    fn sign(&self, data: &[u8]) -> Result<MlDsaSignature, LedgerError> {
        use pqcrypto_traits::sign::DetachedSignature;
        let sig = pqcrypto_mldsa::mldsa65::detached_sign(data, &self.signing_key);
        Ok(MlDsaSignature(sig.as_bytes().to_vec()))
    }

    fn verify(&self, data: &[u8], sig: &MlDsaSignature) -> Result<bool, LedgerError> {
        use pqcrypto_traits::sign::DetachedSignature;
        let detached = pqcrypto_mldsa::mldsa65::DetachedSignature::from_bytes(sig.as_bytes())
            .map_err(|_| LedgerError::VerificationFailed("invalid signature bytes".into()))?;
        match pqcrypto_mldsa::mldsa65::verify_detached_signature(&detached, data, &self.verification_key) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
