#!/usr/bin/env -S nu --stdin

# OpenTofu external key provider: hands the state passphrase to the chained
# `pbkdf2` key provider. Protocol: header line, then read stored meta from
# stdin (`null` on first encryption), then emit base64 keys + meta to store.
def main [] {
    {magic: "OpenTofu-External-Key-Provider", version: 1} | to json -r | print

    $in | ignore
    let passphrase = secretspec get TOFU_STATE_PASSPHRASE | str trim | encode base64

    {
        keys: { encryption_key: $passphrase, decryption_key: $passphrase },
        meta: { external_data: {} },
    } | to json -r | print
}
