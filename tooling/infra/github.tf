data "onepassword_item" "botswain" {
  vault = data.onepassword_vault.wp-prod.uuid
  uuid  = "aj5idhytaxzgn4xbkajbffc4i4"
}

locals {
  botswain-ids = data.onepassword_item.botswain.section_map["ids"].field_map # sugar

  botswain = {
    app-id          = nonsensitive(local.botswain-ids["app"].value)
    installation-id = local.botswain-ids["installation"].value
    private-key     = data.onepassword_item.botswain.private_key
  }
}

provider "github" {
  owner = "jmuchovej" # TODO(jmuchovej): move to `waypoints-hq` once the repo moves

  app_auth {
    id              = local.botswain.app-id
    installation_id = local.botswain.installation-id
    pem_file        = local.botswain.private-key
  }
}

import {
  id = "waypoints"
  to = github_repository.waypoints
}

resource "github_repository" "waypoints" {
  name         = "waypoints"
  description  = "Tools for nonlinear thought."
  homepage_url = "https://waypoints.so"
  visibility   = "public"
  topics       = ["app", "graph-database", "knowledge-management"]

  has_issues      = true
  has_discussions = true
  has_projects    = true
  has_wiki        = false

  allow_merge_commit     = true
  allow_squash_merge     = true
  allow_rebase_merge     = true
  allow_auto_merge       = true
  allow_update_branch    = true
  delete_branch_on_merge = true

  merge_commit_title          = "PR_TITLE"
  merge_commit_message        = "PR_BODY"
  squash_merge_commit_title   = "PR_TITLE"
  squash_merge_commit_message = "BLANK"

  web_commit_signoff_required = true

  lifecycle {
    prevent_destroy = true
  }
}

# Stops anyone hand-minting a tag the release pipelines would act on. Only the
# planners create these, so the App is exempt and nothing else is.
#
# No `deletion` rule: a nightly shortfall deletes the tag it minted under
# GITHUB_TOKEN, which cannot be a bypass actor.
resource "github_repository_ruleset" "protected-releases" {
  name        = "protected-releases"
  repository  = github_repository.waypoints.name
  target      = "tag"
  enforcement = "active"

  conditions {
    ref_name {
      include = ["refs/tags/v*", "refs/tags/*-v*"]
      exclude = []
    }
  }

  bypass_actors {
    actor_id    = tonumber(local.botswain.app-id)
    actor_type  = "Integration"
    bypass_mode = "exempt"
  }

  rules {
    creation         = true
    update           = true
    non_fast_forward = true
  }
}
