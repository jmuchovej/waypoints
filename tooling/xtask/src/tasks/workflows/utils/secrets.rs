//! Steps that decrypt `.secrets/*.sops.yaml` into the job environment.
//!
//! The shell is generated, not written. `::add-mask::` and `$GITHUB_ENV` are
//! runner protocol — a line on stdout and a file appended between steps — so
//! they can only happen at run time, which means bash. What this module removes
//! is *hand-written* bash: every extraction goes through [`Sops`], where
//! masking and exporting are one operation and cannot be written apart.

use gh_workflow::{Run, Step};
use indoc::{formatdoc, indoc};

use super::vars::SOPS_AGE_KEY;

/// Where the runner keeps per-job scratch files.
const TEMP: &str = "${RUNNER_TEMP}";

/// A decrypt step under construction. Each method appends shell; [`Sops::step`]
/// assembles it.
pub struct Sops {
    lines: Vec<String>,
    /// Set by any method emitting a bashism, so no one has to remember that a
    /// desktop job also runs this on Windows, where the default shell is pwsh.
    needs_bash: bool,
}

/// Decrypts `file` into a `secrets` shell variable for the methods below.
pub fn sops(file: &str) -> Sops {
    Sops {
        lines: vec![format!("secrets=$(sops -d {file})")],
        needs_bash: false,
    }
}

impl Sops {
    /// Binds a value to a shell variable and masks it, without exporting.
    ///
    /// For values a later [`Sops::then`] block consumes but no step after this
    /// one should see.
    pub fn secret(mut self, var: &str, path: &str) -> Self {
        self.lines.push(formatdoc! {r#"
            {var}=$(echo "${{secrets}}" | yq '{path}')
            echo "::add-mask::${{{var}}}""#});
        self
    }

    /// Binds, masks, and exports to `$GITHUB_ENV`.
    pub fn export(mut self, name: &str, path: &str) -> Self {
        self.lines.push(formatdoc! {r#"
            {name}=$(echo "${{secrets}}" | yq '{path}')
            echo "::add-mask::${{{name}}}"
            echo "{name}=${{{name}}}" >> "$GITHUB_ENV""#});
        self
    }

    /// As [`Sops::export`], for a value spanning several lines.
    ///
    /// `add-mask` matches exact strings, so a multi-line value has to be masked
    /// line by line, and crosses into `$GITHUB_ENV` as a heredoc.
    pub fn export_multiline(mut self, name: &str, path: &str) -> Self {
        self.needs_bash = true;
        self.lines.push(formatdoc! {r#"
            {name}=$(echo "${{secrets}}" | yq '{path}')
            while IFS= read -r line; do
              [ -n "$line" ] && echo "::add-mask::${{line}}"
            done <<< "${{{name}}}"
            {{
              echo '{name}<<SECRET_EOF'
              echo "${{{name}}}"
              echo 'SECRET_EOF'
            }} >> "$GITHUB_ENV""#});
        self
    }

    /// Writes a value verbatim to `$RUNNER_TEMP/{file}` and exports its path.
    ///
    /// The path is not masked — it is not a secret, and masking it would redact
    /// every mention of the temp directory in the log.
    pub fn text_file(mut self, name: &str, path: &str, file: &str) -> Self {
        self.lines.push(formatdoc! {r#"
            {name}="{TEMP}/{file}"
            echo "${{secrets}}" | yq '{path}' > "${{{name}}}"
            echo "{name}=${{{name}}}" >> "$GITHUB_ENV""#});
        self
    }

    /// base64-decodes a value into `$RUNNER_TEMP/{file}`, binding its path to a
    /// shell variable for a later [`Sops::then`] block.
    pub fn binary_file(mut self, var: &str, path: &str, file: &str) -> Self {
        self.lines.push(formatdoc! {r#"
            {var}="{TEMP}/{file}"
            echo "${{secrets}}" | yq '{path}' | base64 -d > "${{{var}}}""#});
        self
    }

    /// As [`Sops::binary_file`], exporting the path for later steps.
    pub fn binary_file_export(self, name: &str, path: &str, file: &str) -> Self {
        let mut this = self.binary_file(name, path, file);
        this.lines
            .push(format!(r#"echo "{name}=${{{name}}}" >> "$GITHUB_ENV""#));
        this
    }

    /// Raw shell, for the steps that are not a decrypt-mask-export.
    pub fn then(mut self, script: &str) -> Self {
        self.lines.push(script.trim_end().to_owned());
        self
    }

    pub fn step(self, name: &str) -> Step<Run> {
        let step = Step::new(name)
            .run(self.lines.join("\n\n"))
            .add_env(("SOPS_AGE_KEY", SOPS_AGE_KEY));

        match self.needs_bash {
            true => step.shell("bash"),
            false => step,
        }
    }
}

/// Developer ID certificate and notarization key, for signing desktop builds.
pub fn apple_desktop() -> Step<Run> {
    sops(".secrets/apple.sops.yaml")
        .text_file("APPLE_API_KEY_PATH", ".developer-id.key-prv", "AuthKey.p8")
        .export("APPLE_API_ISSUER", ".developer-id.issuer-id")
        .export("APPLE_API_KEY", ".developer-id.key-id")
        .export("APPLE_CERTIFICATE", ".developer-id.certificate")
        .export("APPLE_CERTIFICATE_PASSWORD", ".developer-id.password")
        .step("Decrypt Apple signing secrets")
}

/// The key Tauri's updater signs release manifests with.
pub fn updater() -> Step<Run> {
    sops(".secrets/updater.sops.yaml")
        .export_multiline("TAURI_SIGNING_PRIVATE_KEY", ".private-key")
        .export("TAURI_SIGNING_PRIVATE_KEY_PASSWORD", ".password")
        .step("Decrypt updater signing key")
}

/// Distribution certificate into a throwaway keychain, provisioning profile
/// into Xcode's search path, and the App Store Connect key for fastlane.
pub fn apple_ios() -> Step<Run> {
    sops(".secrets/apple.sops.yaml")
        .binary_file("cert_path", ".distribution.certificate", "distribution.p12")
        .secret("cert_password", ".distribution.password")
        .then(indoc! {r#"
            keychain_password=$(openssl rand -base64 32)
            echo "::add-mask::${keychain_password}"

            keychain_path="${RUNNER_TEMP}/build.keychain-db"
            security create-keychain -p "${keychain_password}" "${keychain_path}"
            security set-keychain-settings -lut 21600 "${keychain_path}"
            security unlock-keychain -p "${keychain_password}" "${keychain_path}"
            security import "${cert_path}" -P "${cert_password}" -A -t cert -f pkcs12 -k "${keychain_path}"
            security set-key-partition-list -S apple-tool:,apple: -k "${keychain_password}" "${keychain_path}"
            security list-keychains -d user -s "${keychain_path}" login.keychain
        "#})
        .binary_file(
            "profile_tmp",
            ".distribution.profile",
            "waypoints.mobileprovision",
        )
        .then(indoc! {r#"
            # A profile must be named by its UUID for Xcode to find it
            profile_path="${HOME}/Library/MobileDevice/Provisioning Profiles"
            mkdir -p "${profile_path}"
            profile_uuid=$(security cms -D -i "${profile_tmp}" | plutil -extract UUID raw -)
            profile_name=$(security cms -D -i "${profile_tmp}" | plutil -extract Name raw -)
            cp "${profile_tmp}" "${profile_path}/${profile_uuid}.mobileprovision"
            echo "Installed profile: ${profile_name} (${profile_uuid})"
        "#})
        .text_file("ASC_KEY_PATH", ".developer-id.key-prv", "AuthKey.p8")
        .export("ASC_KEY_ID", ".developer-id.key-id")
        .export("ASC_ISSUER_ID", ".developer-id.issuer-id")
        .step("Decrypt Apple signing secrets")
}

/// Upload keystore, written where Gradle expects to find it.
pub fn android_signing() -> Step<Run> {
    sops(".secrets/android.sops.yaml")
        .binary_file("keystore_path", ".keystore", "waypoints.jks")
        .secret("key_alias", ".key-alias")
        .secret("password", ".password")
        .then(indoc! {r#"
            cat > waypoints/src-tauri/gen/android/keystore.properties <<EOF
            storeFile=${keystore_path}
            keyAlias=${key_alias}
            password=${password}
            EOF
        "#})
        .step("Decrypt Android signing secrets")
}

/// Service-account JSON that lets fastlane talk to the Play Store.
pub fn play_store() -> Step<Run> {
    sops(".secrets/android.sops.yaml")
        .binary_file_export(
            "GOOGLE_PLAY_JSON_KEY_PATH",
            ".play-store.json-key",
            "play-store-key.json",
        )
        .step("Decrypt Play Store credentials")
}
