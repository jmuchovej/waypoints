//! One shared constructor for reaching the `releases` bucket: resolves the
//! long-lived parent through secretspec's Rust API, derives a short-lived,
//! channel-scoped session credential by local JWT signing, drops the
//! parent, and returns an `opendal` S3 [`Operator`] built from the derived
//! triple alone — see the design's "The store is reached through opendal
//! in-process, from one client constructor" decision.
//!
//! `xtask` shells out only where no Rust API exists (`devenv`, `gh`,
//! `treefmt`); R2 is plain S3 with mature Rust clients, so this reaches it
//! directly rather than through a transfer tool. No caller of
//! [`create_r2_client`] ever sees the parent, and no generated workflow step
//! names a transfer tool — each is a one-line `cargo xtask` dispatch.
//!
//! JWT shape and signing confirmed against Cloudflare's own documented
//! example (<https://developers.cloudflare.com/r2/examples/authenticate-r2-temp-credentials/>):
//! header `{"alg":"HS256","typ":"JWT"}`, claims `sub` (account id), `iss`
//! (parent access key id), `aud` (endpoint host), `iat`/`exp`, `bucket`,
//! `scope`, and `paths.prefixPaths`; signed HS256 with the raw parent secret
//! access key. The derived secret is the hex SHA-256 digest of the signed
//! JWT; the session token is `base64("jwt/" + <signed JWT>)`.

use crate::tasks::workflows::Channel;
use anyhow::{Context as _, Result, bail};
use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD as BASE64_STANDARD, URL_SAFE_NO_PAD},
};
use hmac::{Hmac, KeyInit, Mac};
use opendal::{Operator, layers::RetryLayer, services::S3};
use secretspec::{NamedResolution, Secrets};
use sha2::{Digest, Sha256};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 3 hours: comfortably longer than any current build and far short of R2's
/// 7-day ceiling. Configurable because a new dependency or a cold cache can
/// lengthen a compile without warning, and a credential expiring mid-upload
/// is a failed release for no good reason.
pub const DEFAULT_TTL_SECONDS: u64 = 3 * 60 * 60;

const BUCKET: &str = "releases";

/// Writes objects without administering the bucket — the job has no
/// business altering bucket configuration.
const PERMISSION: &str = "object-read-write";

/// The root-level key a channel's manifest lives at — `nightly.json`,
/// `preview.json`, `stable.json`. Shared between the credential that must
/// grant exact access to it and the manifest-write step that writes it.
pub fn manifest_key(channel: Channel) -> String {
    format!("{}.json", channel.flavor())
}

/// The releases bucket's long-lived parent, resolved once and dropped as
/// soon as [`create_r2_client`] has derived the session credential from it.
/// Nothing downstream of that point ever holds it.
struct ParentCredential {
    access_key_id: String,
    secret_access_key: String,
    endpoint: String,
}

/// The derived, channel-scoped triple a store client is built from.
struct SessionCredential {
    access_key_id: String,
    secret_access_key: String,
    session_token: String,
    endpoint: String,
}

/// Installs the process-wide HTTP transport and TLS crypto provider the
/// client needs, exactly once. `opendal`'s own `install_default` only wires
/// up the plain `rustls` feature variant, not `rustls-no-provider`, so this
/// calls the transport crate directly instead. Idempotent and safe from any
/// number of callers — including a test binary, which never runs
/// `bin/xtask.rs`'s `main` and so cannot rely on a one-time setup step
/// there.
fn install_http_stack() {
    static INSTALL: std::sync::Once = std::sync::Once::new();
    INSTALL.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
        opendal_http_transport_reqwest::install_default();
    });
}

/// Resolves the parent, derives a session credential scoped to `channel`'s
/// key prefix, drops the parent, and returns an `opendal` `Operator` for the
/// `releases` bucket built from the derived triple alone.
///
/// Scoped to two things: every key under `{channel}/` (the artifacts this
/// run uploads or, on a shortfall, deletes) and the single root-level
/// `{channel}.json` key (the manifest this channel's publish step writes
/// last). The manifest's key does not start with the channel's own prefix,
/// so without naming it as an exact object path the upload credential could
/// never write it — R2's local-signing scheme grants prefixes and
/// individual objects as two separate lists for exactly this reason.
pub fn create_r2_client(channel: Channel, ttl_seconds: u64) -> Result<Operator> {
    install_http_stack();

    let credential = {
        let parent = resolve_parent()?;
        let prefix = format!("{}/", channel.flavor());
        let manifest_key = manifest_key(channel);
        derive_session_credential(
            &parent,
            BUCKET,
            &prefix,
            &[&manifest_key],
            PERMISSION,
            Duration::from_secs(ttl_seconds),
            SystemTime::now(),
        )?
        // `parent` drops here, at the end of this block — before the
        // builder below, let alone the operator it produces, exists.
    };

    build_operator(&credential)
}

/// Builds the `Operator` from an already-derived session credential. Split
/// out from [`create_r2_client`] so a test can construct one from a
/// synthetic credential and inspect the result without a vault.
///
/// `disable_config_load` and `disable_ec2_metadata` keep the client from
/// reading ambient `AWS_*` variables or probing EC2 instance metadata —
/// opendal does both by default, which would let the environment override a
/// credential this deliberately scopes. `region("auto")` matches the other
/// R2 bucket this repo already talks to (the build cache's `sccache`
/// configuration). `enable_exact_buf_write`, R2's old fix for mismatched
/// multipart part sizes, is gone from opendal's S3 builder as of 0.59 —
/// superseded by `BufferedWriter` — and does not apply to the single
/// `write_with` call this client makes in any case.
fn build_operator(credential: &SessionCredential) -> Result<Operator> {
    let builder = S3::default()
        .endpoint(&credential.endpoint)
        .bucket(BUCKET)
        .region("auto")
        .access_key_id(&credential.access_key_id)
        .secret_access_key(&credential.secret_access_key)
        .session_token(&credential.session_token)
        .disable_config_load()
        .disable_ec2_metadata();

    Ok(Operator::new(builder)
        .context("building the releases bucket operator")?
        .layer(RetryLayer::new()))
}

/// Resolves the three `releases` scope secrets through secretspec's Rust
/// API — in-process, so the parent never becomes an OS environment variable
/// even transiently, unlike every other credential this repo's CI resolves.
fn resolve_parent() -> Result<ParentCredential> {
    let mut spec = Secrets::load().context("loading secretspec.toml")?;
    spec.set_profile("release");
    spec.set_scope("releases");
    let spec = spec.with_reason(
        "xtask mints a short-lived, channel-scoped R2 session credential for this release upload",
    );

    Ok(ParentCredential {
        access_key_id: resolve_named(&spec, "R2_RELEASES_ACCESS_KEY_ID")?,
        secret_access_key: resolve_named(&spec, "R2_RELEASES_SECRET_ACCESS_KEY")?,
        endpoint: resolve_named(&spec, "R2_RELEASES_ENDPOINT")?,
    })
}

fn resolve_named(spec: &Secrets, name: &str) -> Result<String> {
    match spec
        .resolve_named(name)
        .with_context(|| format!("resolving {name}"))?
    {
        NamedResolution::Resolved(secret) => secret
            .value
            .with_context(|| format!("{name} resolved with no inline value")),
        NamedResolution::Missing { required } => {
            bail!("{name} is missing from the vault (required: {required})")
        }
        NamedResolution::Undeclared => {
            bail!("{name} is not declared in the release profile's releases scope")
        }
    }
}

/// The host segment of an R2 S3 endpoint URL, with any scheme and trailing
/// slash stripped — e.g. `https://{account_id}.r2.cloudflarestorage.com` to
/// `{account_id}.r2.cloudflarestorage.com`.
fn endpoint_host(endpoint: &str) -> Result<String> {
    let host = endpoint
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');
    if host.is_empty() {
        bail!("empty R2 endpoint");
    }
    Ok(host.to_owned())
}

/// The account id is the endpoint host's leading label — R2's own addressing
/// scheme, not a value this task is handed separately.
fn account_id(host: &str) -> Result<&str> {
    let id = host.split('.').next().filter(|label| !label.is_empty());
    id.with_context(|| format!("{host} has no leading account-id label"))
}

/// Builds and HS256-signs the claims JWT, then derives the session
/// credential from it: the secret is the hex SHA-256 digest of the signed
/// JWT, the access key id is reused from the parent, and the session token
/// is `base64("jwt/" + <signed JWT>)`.
fn derive_session_credential(
    parent: &ParentCredential,
    bucket: &str,
    prefix: &str,
    object_paths: &[&str],
    permission: &str,
    ttl: Duration,
    issued_at: SystemTime,
) -> Result<SessionCredential> {
    let host = endpoint_host(&parent.endpoint)?;
    let account_id = account_id(&host)?;

    let issued_at = issued_at
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?
        .as_secs();
    let expires_at = issued_at + ttl.as_secs();

    let header = serde_json::json!({ "alg": "HS256", "typ": "JWT" });
    let claims = serde_json::json!({
        "sub": account_id,
        "iss": parent.access_key_id,
        "aud": host,
        "iat": issued_at,
        "exp": expires_at,
        "bucket": bucket,
        "scope": permission,
        "paths": { "prefixPaths": [prefix], "objectPaths": object_paths },
    });

    let signing_input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header)?),
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims)?),
    );

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(parent.secret_access_key.as_bytes())
        .expect("hmac takes any key length");
    mac.update(signing_input.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

    let jwt = format!("{signing_input}.{signature}");

    Ok(SessionCredential {
        access_key_id: parent.access_key_id.clone(),
        secret_access_key: hex::encode(Sha256::digest(jwt.as_bytes())),
        session_token: BASE64_STANDARD.encode(format!("jwt/{jwt}")),
        endpoint: parent.endpoint.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn parent() -> ParentCredential {
        ParentCredential {
            access_key_id: "parent-key-id".to_owned(),
            secret_access_key: "parent-secret".to_owned(),
            endpoint: "https://abc123accountid.r2.cloudflarestorage.com".to_owned(),
        }
    }

    /// Splits a derived session token back into its JWT header and claims,
    /// for assertions that do not merely restate the function under test.
    /// Also returns the exact signing input (`header.payload`, still
    /// base64url) and the signature segment, for the independent HMAC check.
    fn decode(credential: &SessionCredential) -> (Value, Value, String, String) {
        let decoded = BASE64_STANDARD.decode(&credential.session_token).unwrap();
        let decoded = String::from_utf8(decoded).unwrap();
        let jwt = decoded.strip_prefix("jwt/").expect("jwt/ prefix");

        let mut parts = jwt.splitn(3, '.');
        let header_b64 = parts.next().unwrap();
        let payload_b64 = parts.next().unwrap();
        let signature = parts.next().unwrap().to_owned();
        assert!(parts.next().is_none(), "{jwt}");

        let header: Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(header_b64).unwrap()).unwrap();
        let claims: Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload_b64).unwrap()).unwrap();
        let signing_input = format!("{header_b64}.{payload_b64}");
        (header, claims, signing_input, signature)
    }

    #[test]
    fn derives_a_well_formed_session_credential() {
        let issued_at = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let credential = derive_session_credential(
            &parent(),
            "releases",
            "nightly/",
            &["nightly.json"],
            "object-read-write",
            Duration::from_secs(10_800),
            issued_at,
        )
        .unwrap();

        assert_eq!(credential.access_key_id, "parent-key-id");
        assert_eq!(credential.endpoint, parent().endpoint);
        assert_eq!(credential.secret_access_key.len(), 64, "hex sha256 digest");
        assert!(
            credential
                .secret_access_key
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "{}",
            credential.secret_access_key
        );

        let (header, claims, signing_input, signature) = decode(&credential);
        assert_eq!(header, serde_json::json!({ "alg": "HS256", "typ": "JWT" }));
        assert_eq!(claims["sub"], "abc123accountid");
        assert_eq!(claims["iss"], "parent-key-id");
        assert_eq!(claims["aud"], "abc123accountid.r2.cloudflarestorage.com");
        assert_eq!(claims["iat"], 1_700_000_000);
        assert_eq!(claims["exp"], 1_700_000_000 + 10_800);
        assert_eq!(claims["bucket"], "releases");
        assert_eq!(claims["scope"], "object-read-write");
        assert_eq!(
            claims["paths"]["prefixPaths"],
            serde_json::json!(["nightly/"])
        );
        assert_eq!(
            claims["paths"]["objectPaths"],
            serde_json::json!(["nightly.json"])
        );

        // Re-derive the HMAC independently of `derive_session_credential`, so
        // this does not merely assert the function agrees with itself.
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(parent().secret_access_key.as_bytes()).unwrap();
        mac.update(signing_input.as_bytes());
        let expected = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        assert_eq!(signature, expected);
    }

    #[test]
    fn scopes_to_the_given_channel_prefix_and_its_manifest_key_only() {
        for (prefix, manifest) in [
            ("nightly/", "nightly.json"),
            ("preview/", "preview.json"),
            ("stable/", "stable.json"),
        ] {
            let credential = derive_session_credential(
                &parent(),
                "releases",
                prefix,
                &[manifest],
                "object-read-write",
                Duration::from_secs(60),
                SystemTime::now(),
            )
            .unwrap();
            let (_, claims, ..) = decode(&credential);
            assert_eq!(
                claims["paths"]["prefixPaths"],
                serde_json::json!([prefix]),
                "{prefix}"
            );
            assert_eq!(
                claims["paths"]["objectPaths"],
                serde_json::json!([manifest]),
                "{manifest}"
            );
        }
    }

    #[test]
    fn lifetime_follows_the_requested_ttl() {
        let issued_at = SystemTime::now();
        let credential = derive_session_credential(
            &parent(),
            "releases",
            "nightly/",
            &["nightly.json"],
            "object-read-write",
            Duration::from_secs(60),
            issued_at,
        )
        .unwrap();
        let (_, claims, ..) = decode(&credential);
        let iat = claims["iat"].as_u64().unwrap();
        let exp = claims["exp"].as_u64().unwrap();
        assert_eq!(exp - iat, 60);
    }

    /// The default lifetime a bare `create_r2_client(channel,
    /// DEFAULT_TTL_SECONDS)` call carries is the same 3 hours the design
    /// settles on; every caller passes it explicitly, so there is no CLI
    /// default to drift from this constant.
    #[test]
    fn the_default_lifetime_is_three_hours() {
        assert_eq!(DEFAULT_TTL_SECONDS, 3 * 60 * 60);
    }

    /// `build_operator` constructs successfully from a synthetic credential,
    /// without touching the vault — `create_r2_client` only reaches it after
    /// resolving the parent.
    #[test]
    fn build_operator_succeeds_from_a_synthetic_credential() {
        let credential = SessionCredential {
            access_key_id: "AKIDEXAMPLE".to_owned(),
            secret_access_key: "secret".to_owned(),
            session_token: "token".to_owned(),
            endpoint: "https://abc123accountid.r2.cloudflarestorage.com".to_owned(),
        };
        build_operator(&credential).unwrap();
    }

    /// opendal's S3 builder reads ambient `AWS_*` variables and probes EC2
    /// instance metadata unless told not to — exactly the kind of implicit
    /// environment override a credential this deliberately scoped must
    /// refuse. The config is not otherwise introspectable once built (its
    /// fields are private to the service crate), so this pins the call
    /// sites in source the same way `only_the_wrapper_constructor_names_rustc_wrapper`
    /// (in `_steps.rs`) pins its own invariant.
    #[test]
    fn both_disable_flags_are_set_exactly_once() {
        // Excludes this test module's own source, which would otherwise
        // match its own search strings below.
        let source = include_str!("release_store.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        assert_eq!(production.matches(".disable_config_load()").count(), 1);
        assert_eq!(production.matches(".disable_ec2_metadata()").count(), 1);
    }

    /// Task 2.3's live verification: a derived credential can write under
    /// its own channel prefix, cannot write under another channel's, a
    /// second write to the same key is refused rather than overwriting, and
    /// a credential outliving its own TTL is refused. Needs real vault
    /// access and network access to the live bucket, so it is `#[ignore]`d
    /// by default — run it explicitly:
    ///
    /// ```sh
    /// SECRETSPEC_PROVIDER="onepassword://wp-prod" cargo test -p xtask --lib \
    ///   tasks::release_store::tests::live_credential_is_scoped_atomic_and_expiring \
    ///   -- --ignored --nocapture
    /// ```
    ///
    /// Cleans up the one key it writes and leaves on success; a failure
    /// partway through may leave it behind (harmless: it is an
    /// unreferenced, unadvertised object under a path no manifest names).
    #[test]
    #[ignore = "requires network access and a real vault credential"]
    fn live_credential_is_scoped_atomic_and_expiring() {
        let runtime = tokio::runtime::Runtime::new().expect("build a tokio runtime");
        runtime.block_on(async {
            let own_key = "nightly/manual-verification/live-check.txt";
            let other_channel_key = "stable/manual-verification/live-check.txt";

            let client =
                create_r2_client(Channel::Nightly, 300).expect("mint a nightly-scoped client");

            client
                .write_with(own_key, b"live verification".to_vec())
                .if_not_exists(true)
                .await
                .expect("writes under its own channel prefix");

            let repeat = client
                .write_with(own_key, b"second write".to_vec())
                .if_not_exists(true)
                .await;
            assert!(
                repeat.is_err(),
                "a second write to the same key must be refused, not overwrite it"
            );

            let cross_channel = client
                .write_with(other_channel_key, b"should fail".to_vec())
                .if_not_exists(true)
                .await;
            assert!(
                cross_channel.is_err(),
                "a nightly-scoped credential must not write under stable/"
            );

            client
                .delete(own_key)
                .await
                .expect("cleans up the key this test wrote");

            let short_lived =
                create_r2_client(Channel::Nightly, 2).expect("mint a short-lived client");
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            let expired_write = short_lived
                .write_with(
                    "nightly/manual-verification/should-not-land.txt",
                    b"x".to_vec(),
                )
                .if_not_exists(true)
                .await;
            assert!(
                expired_write.is_err(),
                "a credential outliving its configured TTL must be refused"
            );
        });
    }
}
