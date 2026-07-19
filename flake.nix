{
  description = "tender-db — an API and browser for public tenders (Dioxus 0.7 fullstack: Axum server + Dioxus WASM client bundle)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # Rust toolchains (gives us a pinned stable rustc/cargo plus the
    # wasm32-unknown-unknown target needed to build the web client).
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # Builds Cargo workspaces with vendored, cached dependencies.
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      fenix,
      crane,
    }:
    let
      inherit (nixpkgs) lib;

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      forAllSystems = f: lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      # All tender-db artifacts ({ server; devShell; }) for one nixpkgs.
      tenderDbFor =
        pkgs:
        import ./nix/package.nix {
          inherit pkgs fenix crane;
          src = ./.;
        };
    in
    {
      # Overlay that adds the fullstack server bundle to pkgs.
      overlays.default =
        final: prev:
        {
          tender-db = (tenderDbFor final).server;
        };

      packages = forAllSystems (
        pkgs:
        let
          tender-db = tenderDbFor pkgs;
        in
        {
          tender-db = tender-db.server;
          default = tender-db.server;
        }
      );

      # NixOS module: runs the server as a hardened, sandboxed systemd service.
      nixosModules.default = import ./nix/module.nix self;

      devShells = forAllSystems (pkgs: {
        default = (tenderDbFor pkgs).devShell;
      });

      checks = forAllSystems (
        pkgs:
        {
          # `cargo clippy` as a build-time lint gate.
          clippy = (tenderDbFor pkgs).clippy;
        }
        // lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          # Boots a VM with the module enabled and probes the HTTP endpoints.
          vm-smoke = pkgs.testers.runNixOSTest (import ./nix/vm-smoke-test.nix self);
        }
      );

      formatter = forAllSystems (pkgs: pkgs.nixfmt);
    };
}
