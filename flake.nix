# catch — the team PR-review hot-potato game engine (batata-quente codename).
#
# Build pattern: the ONE pleme-io Rust pattern — substrate's gen-driven
# lockfile-builder.mkProject over the committed Cargo.gen.lock delta + a
# dockerTools image hardened via substrate's oci/hardened-base.nix
# (distroless-glibc, no shell, nonroot-by-default — Pillar 8, no
# Dockerfile). Mirrors breathe. After any Cargo.lock change: `gen build .`
# (re-emits Cargo.gen.lock committed + Cargo.build-spec.json gitignored).
#
# CI/CD: the ESTEIRA private rio lane (theory/ESTEIRA.md) — catch is a private
# repo, so its conveyance runs on [self-hosted, rio] and `nix run .#release`
# pushes to ghcr.io/pleme-io/catch via the nix closure (skopeo, no docker),
# the pitr-tools reference. `.#release <tag>` is the rio-lane release app.
{
  description = "catch — the team PR-review hot-potato game engine";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    substrate = {
      url = "github:pleme-io/substrate";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, substrate }:
    flake-utils.lib.eachSystem [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ] (system: let
      pkgs = import nixpkgs { inherit system; };
      version = "0.1.0";

      plemeCrateOverrides = import "${substrate}/lib/build/rust/pleme-crate-overrides.nix";
      lockfileBuilder = import "${substrate}/lib/build/rust/lockfile-builder.nix" { inherit pkgs; };
      # catch-api-server's build.rs compiles proto/catch.proto via
      # tonic-build → prost-build, which shells out to `protoc`. The
      # hermetic build sandbox has no protoc, so the proto-compiling
      # member declares its build-time tool need here — the gen-pattern
      # analog of buildRustPackage's nativeBuildInputs = [ protobuf ].
      # Every URDUME service has a tonic face, so this recurs fleet-wide.
      protocOverride = {
        catch-api-server = attrs: {
          nativeBuildInputs = (attrs.nativeBuildInputs or [ ]) ++ [ pkgs.protobuf ];
          PROTOC = "${pkgs.protobuf}/bin/protoc";
        };
      };
      project = lockfileBuilder.mkProject {
        src = self;
        defaultCrateOverrides = pkgs.defaultCrateOverrides // plemeCrateOverrides // protocOverride;
      };

      apiServer = project.workspaceMembers.catch-api-server.build;

      # Pillar 8 — dockerTools, no Dockerfile. Hardened via substrate's
      # oci/hardened-base.nix (distroless-glibc — no shell, TLS roots +
      # nonroot /etc/passwd|group stub, nonroot 65532:65532 by default)
      # instead of a hand-rolled dockerTools.buildLayeredImage — the SAME
      # hardened substrate breathe's flake.nix and tool-image.nix build
      # against. catch-api-server is a dynamically-linked glibc binary (no
      # musl target in this flake), so distroless-glibc is correct.
      hardened = import "${substrate}/lib/build/oci/hardened-base.nix" { inherit pkgs; };
      image =
        if pkgs.stdenv.isLinux then
          hardened.mkPackageImage {
            service = "catch";
            base = hardened.bases.distroless-glibc;
            package = apiServer;
            publishName = "ghcr.io/pleme-io/catch";
            publishTag = version;
            entrypoint = [ "${apiServer}/bin/catch-api-server" ];
            exposedPorts = { "8080/tcp" = {}; };
            env = [
              "SSL_CERT_FILE=/etc/ssl/certs/ca-bundle.crt"
              "RUST_LOG=info,catch_api_server=info"
              "CATCH_API_BIND=0.0.0.0:8080"
            ];
          }
        else
          pkgs.runCommand "catch-image-darwin-stub" {} ''
            mkdir -p $out
            echo "Build the OCI image on Linux: nix build .#image --system x86_64-linux" > $out/README
          '';

      # The ESTEIRA rio-lane release app: `nix run .#release -- <tag>` pushes the
      # dockerTools image to ghcr via skopeo from the nix closure (no docker, no
      # `skopeo login` — a v2 registries.conf + --dest-creds, the proven rio path).
      # Creds: GHCR_USER + GHCR_TOKEN env (the rio runner's SOPS-sourced token).
      releaseApp = pkgs.writeShellApplication {
        name = "catch-release";
        runtimeInputs = [ pkgs.skopeo pkgs.nix ];
        text = ''
          tag="''${1:?usage: release <tag>}"
          : "''${GHCR_USER:?set GHCR_USER}" "''${GHCR_TOKEN:?set GHCR_TOKEN}"
          conf="$(mktemp)"; printf 'unqualified-search-registries = ["docker.io"]\n' > "$conf"
          img="$(nix build .#image --no-link --print-out-paths)"
          skopeo --registries-conf "$conf" copy \
            --dest-creds "$GHCR_USER:$GHCR_TOKEN" \
            "docker-archive:$img" "docker://ghcr.io/pleme-io/catch:$tag"
          echo "pushed ghcr.io/pleme-io/catch:$tag"
        '';
      };

    in {
      packages = {
        default = apiServer;
        catch-api-server = apiServer;
        image = image;
      };
      apps.default = { type = "app"; program = "${apiServer}/bin/catch-api-server"; };
      apps.release = { type = "app"; program = "${releaseApp}/bin/catch-release"; };
      devShells.default = pkgs.mkShellNoCC {
        buildInputs = with pkgs; [ rustc cargo pkg-config skopeo ];
      };
    });
}
