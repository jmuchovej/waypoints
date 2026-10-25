module "r2-ci-cache" {
  source = "./cloudflare/r2"

  account-id  = local.account-id
  bucket-name = "ci-cache"
}
