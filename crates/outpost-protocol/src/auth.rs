//! Connection identity. A device key binds the transport
//! connection (the `VesselID`); a [`DeviceCert`] is the identity's signed
//! statement that the device key is one of its own. On connect, a peer
//! presents `(identity, cert)` and the other side checks the cert *and* that
//! the certified device key is the connection's authenticated key — learning
//! which identity is behind the device without trusting the claim.
//!
//! The type lives here (not in `outpost-identity`) so it can travel in a
//! [`SyncMessage`](crate::sync::SyncMessage) handshake; the identity crate
//! constructs it.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

/// Domain tag separating device-certification signatures from every other
/// use of the identity key.
pub const DEVICE_CERT_CONTEXT: &[u8] = b"outpost/vessel-cert/v1";

/// The message an identity signs to certify a device key.
pub fn cert_message(device: &VerifyingKey) -> Vec<u8> {
    let mut message = Vec::with_capacity(DEVICE_CERT_CONTEXT.len() + 32);
    message.extend_from_slice(DEVICE_CERT_CONTEXT);
    message.extend_from_slice(device.as_bytes());
    message
}

/// An identity-signed statement "this device key is mine".
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCert {
    pub device: VerifyingKey,
    pub signature: Signature,
}

impl DeviceCert {
    /// Certify `device` as belonging to `identity`.
    pub fn issue(identity: &SigningKey, device: &VerifyingKey) -> Self {
        Self {
            device: *device,
            signature: identity.sign(&cert_message(device)),
        }
    }

    /// Verify this certificate against a claimed identity public key.
    pub fn verify(&self, identity: &VerifyingKey) -> Result<(), ed25519_dalek::SignatureError> {
        identity.verify_strict(&cert_message(&self.device), &self.signature)
    }
}
