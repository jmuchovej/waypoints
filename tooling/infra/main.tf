terraform {
  required_providers {
    cloudflare  = { source = "cloudflare/cloudflare", version = "~> 5.27.0" }
    onepassword = { source = "1password/onepassword", version = "~> 3.3.1" }
  }

  backend "pg" {}

  encryption {
    key_provider "external" "op" {
      command = ["./bin/op-provider.nu"]
    }

    key_provider "pbkdf2" "default" {
      chain         = key_provider.external.op
      key_length    = 32
      iterations    = 600000
      salt_length   = 32
      hash_function = "sha512"
    }

    method "aes_gcm" "default" {
      keys = key_provider.pbkdf2.default
    }

    state {
      method   = method.aes_gcm.default
      enforced = true
    }

    plan {
      method   = method.aes_gcm.default
      enforced = true
    }
  }
}

provider "onepassword" {
  account = "SNDQCTZFLVHCTA6ILHCIFZJCO4"
}

data "onepassword_vault" "waypoints" {
  name = "waypoints.so"
}

data "onepassword_environment" "waypoints" {
  environment_id = "icuw44lgx42gmcmslnjcbl5tp4"
}

locals {
  account-id = data.onepassword_environment.waypoints.variables["CLOUDFLARE_ACCOUNT_ID"]
}

provider "cloudflare" {
  api_token = data.onepassword_environment.waypoints.variables["CLOUDFLARE_API_TOKEN"]
}
