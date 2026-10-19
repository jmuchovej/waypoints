//! Derives a short-lived, channel-scoped R2 session credential for the
//! `releases` bucket from its long-lived parent, by local JWT signing rather
//! than a call to R2's temporary-credentials endpoint — see the design's
//! "Upload credentials are minted locally" decision. The parent is resolved
//! once, in-process, through secretspec's Rust API; nothing that contacts
//! the store ever sees it, and only the derived values reach `$GITHUB_OUTPUT`.
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
use clap::Parser;
use hmac::{Hmac, KeyInit, Mac};
use secretspec::{NamedResolution, Secrets};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 3 hours: comfortably longer than any current build and far short of R2's
/// 7-day ceiling. Configurable because a new dependency or a cold cache can
/// lengthen a compile without warning, and a credential expiring mid-upload
/// is a failed release for no good reason.
const DEFAULT_TTL_SECONDS: u64 = 3 * 60 * 60;

const BUCKET: &str = "releases";

/// Writes objects without administering the bucket — the job has no
/// business altering bucket configuration.
const PERMISSION: &str = "object-read-write";

#[derive(Parser)]
pub struct ReleaseCredentialArgs {
    /// The channel this run is publishing. The derived credential is
    /// confined to this channel's key prefix alone.
    #[arg(long, value_enum)]
    channel: Channel,

    /// How long the derived credential stays valid, in seconds.
    #[arg(long, default_value_t = DEFAULT_TTL_SECONDS)]
    ttl_seconds: u64,
}

/// The releases bucket's long-lived parent, resolved once and never
/// exported — only [`derive_session_credential`]'s output is.
struct ParentCredential {
    access_key_id: String,
    secret_access_key: String,
    endpoint: String,
}

/// What a release job's upload and manifest steps receive. Every field is a
/// scalar, so it serializes straight into `$GITHUB_OUTPUT`.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct SessionCredential {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: String,
    pub endpoint: String,
}

pub fn run(args: ReleaseCredentialArgs) -> Result<()> {
    let parent = resolve_parent()?;
    let prefix = format!("{}/", args.channel.flavor());
    let credential = derive_session_credential(
        &parent,
        BUCKET,
        &prefix,
        PERMISSION,
        Duration::from_secs(args.ttl_seconds),
        SystemTime::now(),
    )?;

    // The parent never reaches here in anything but `resolve_parent`'s local
    // scope; these two are the only derived values worth hiding from the run
    // log, since the access key id is not secret and the endpoint is public.
    mask(&credential.secret_access_key);
    mask(&credential.session_token);

    crate::utils::github::set_outputs(&credential)
}

/// A GitHub Actions workflow command that redacts `value` from the run log
/// wherever it appears from this point on.
fn mask(value: &str) {
    println!("::add-mask::{value}");
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
        "paths": { "prefixPaths": [prefix], "objectPaths": [] },
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
        assert_eq!(claims["paths"]["objectPaths"], serde_json::json!([]));

        // Re-derive the HMAC independently of `derive_session_credential`, so
        // this does not merely assert the function agrees with itself.
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(parent().secret_access_key.as_bytes()).unwrap();
        mac.update(signing_input.as_bytes());
        let expected = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        assert_eq!(signature, expected);
    }

    #[test]
    fn scopes_to_the_given_channel_prefix_only() {
        for prefix in ["nightly/", "preview/", "stable/"] {
            let credential = derive_session_credential(
                &parent(),
                "releases",
                prefix,
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
        }
    }

    #[test]
    fn lifetime_follows_the_requested_ttl() {
        let issued_at = SystemTime::now();
        let credential = derive_session_credential(
            &parent(),
            "releases",
            "nightly/",
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

    /// The CLI default is the same 3 hours the design settles on; raising it
    /// is a flag, not a code change.
    #[test]
    fn the_cli_defaults_to_a_three_hour_lifetime() {
        let args = ReleaseCredentialArgs::parse_from(["xtask", "--channel", "nightly"]);
        assert_eq!(args.ttl_seconds, 3 * 60 * 60);
    }

    #[test]
    fn a_configured_lifetime_overrides_the_default() {
        let args = ReleaseCredentialArgs::parse_from([
            "xtask",
            "--channel",
            "stable",
            "--ttl-seconds",
            "600",
        ]);
        assert_eq!(args.ttl_seconds, 600);
    }
}
