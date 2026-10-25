terraform {
  required_providers {
    cloudflare  = { source = "cloudflare/cloudflare" }
    onepassword = { source = "1password/onepassword" }
  }
}

variable "account-id" {
  type        = string
  description = "Cloudflare Account ID"
  sensitive   = true
}

variable "bucket-name" {
  type        = string
  description = "Cloudflare R2 Bucket Name"
}

variable "jurisdiction" {
  type        = string
  description = "Default jurisdiction for this R2 Bucket"
  default     = "default"
}

variable "default-storage-class" {
  type        = string
  description = "Default `Storage Class` for this R2 Bucket"
  default     = "Standard"
}

data "cloudflare_account_api_token_permission_groups_list" "r2" {
  account_id = var.account-id
  scope      = "com.cloudflare.edge.r2.bucket"
}

resource "cloudflare_r2_bucket" "this" {
  account_id    = var.account-id
  name          = var.bucket-name
  jurisdiction  = "default"
  storage_class = var.default-storage-class
}

locals {
  bucket-scoping = join("_", [
    "com.cloudflare.edge.r2.bucket.${var.account-id}",
    cloudflare_r2_bucket.this.jurisdiction,
    cloudflare_r2_bucket.this.name,
  ])
}

resource "cloudflare_account_token" "this" {
  account_id = var.account-id
  name       = "r2-${cloudflare_r2_bucket.this.name}"

  policies = [{
    effect = "allow"
    permission_groups = [
      for pg in data.cloudflare_account_api_token_permission_groups_list.r2.result :
      { id = pg.id }
    ]
    resources = jsonencode({
      "${local.bucket-scoping}" = "*"
    })
  }]
}

output "account-id" {
  value     = var.account-id
  sensitive = true
}
output "name" {
  value = cloudflare_r2_bucket.this.name
}
output "jurisdiction" {
  value = cloudflare_r2_bucket.this.jurisdiction
}
