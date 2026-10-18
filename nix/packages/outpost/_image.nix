# The Station as a container image, built by Nix from the `outpost` package
# — no Dockerfile. Exposed by the flake for Linux systems only, never by
# the overlay (the underscore marks a file that is not an overlay package).
#
# What the image promises (see openspec distribution/station-deployment):
# `outpost` is the entrypoint and `station` the default command, so running
# it with a volume at `/data` and no arguments starts a Station that
# persists there; it runs as `nobody` (65534) and writes nowhere but
# `/data`, so a read-only root filesystem works; and it carries no shell,
# no CA bundle, and no `/etc/passwd` (the binary resolves no user names) —
# the Station needs no egress to serve.
#
# Port 6782 is declared for both protocols: QUIC over UDP and the probe
# routes over TCP (design D5/D12). `/data` is created by whoever mounts the
# volume; the manifests' `fsGroup` makes it writable.
{
  dockerTools,
  outpost,
}:
dockerTools.buildLayeredImage {
  name = "outpost";
  tag = outpost.version;
  contents = [ outpost ];
  config = {
    # Entrypoint + Cmd rather than one Entrypoint, so an orchestrator's
    # `args: [station, …]` replaces `station` instead of repeating it.
    Entrypoint = [ "${outpost}/bin/outpost" ];
    Cmd = [ "station" ];
    Env = [ "OUTPOST_STATION_DATA=/data" ];
    User = "65534:65534";
    Volumes."/data" = { };
    ExposedPorts = {
      "6782/udp" = { };
      "6782/tcp" = { };
    };
  };
}
