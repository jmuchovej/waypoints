// Android App Links association for https://waypoints.so/pair. The
// signing-cert fingerprints come from runtimeConfig
// (NUXT_APP_ASSOCIATION_ANDROID_CERT_FINGERPRINTS, comma-separated) so
// pinning or rotating the release cert is a deploy-time change, not a
// commit. Until they are set this serves an empty list, which simply
// leaves the association unverified.

export default defineEventHandler((event) => {
  const { androidCertFingerprints } = useRuntimeConfig(event).appAssociation;
  const fingerprints = androidCertFingerprints
    .split(",")
    .map((f: string) => f.trim())
    .filter(Boolean);

  return [
    {
      relation: ["delegate_permission/common.handle_all_urls"],
      target: {
        namespace: "android_app",
        package_name: "so.waypoints",
        sha256_cert_fingerprints: fingerprints,
      },
    },
  ];
});
