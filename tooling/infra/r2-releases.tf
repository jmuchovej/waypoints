module "r2-releases" {
  source = "./cloudflare/r2"

  account-id  = local.account-id
  bucket-name = "releases"
}

# Release artifacts are archival: they tier down to cheaper storage, never expire.
# Channel leads the object key because these conditions only match on prefix.
locals {
  releases-transition-days = {
    nightly = 30
    preview = 90
    stable  = 270
  }
}

resource "cloudflare_r2_bucket_lifecycle" "releases" {
  account_id   = module.r2-releases.account-id
  bucket_name  = module.r2-releases.name
  jurisdiction = module.r2-releases.jurisdiction

  rules = [
    for channel, days in local.releases-transition-days : {
      id      = "${channel}-to-infrequent-access"
      enabled = true

      conditions = {
        prefix = "${channel}/"
      }

      storage_class_transitions = [{
        storage_class = "InfrequentAccess"
        condition = {
          type = "Age"
          # max_age is seconds, not days.
          max_age = days * 24 * 60 * 60
        }
      }]
    }
  ]
}
