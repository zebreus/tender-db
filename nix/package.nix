# Builds tender-db, a Dioxus 0.7 fullstack app. `dx bundle` produces a single
# runnable bundle containing the native Axum server binary plus the `public/`
# directory of web assets (the WASM client, hashed assets, index.html).
#
# Returned value is an attrset of the artifacts:
#   { server; devShell; }
{
  pkgs,
  fenix,
  crane,
  src,
}:
let
  inherit (pkgs) lib;
  system = pkgs.stdenv.hostPlatform.system;

  # A stable toolchain that can target both the host and wasm32.
  fenixPkgs = fenix.packages.${system};
  toolchain = fenixPkgs.combine [
    fenixPkgs.stable.rustc
    fenixPkgs.stable.cargo
    fenixPkgs.stable.clippy
    fenixPkgs.stable.rustfmt
    fenixPkgs.targets.wasm32-unknown-unknown.stable.rust-std
  ];

  craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

  # Only the files the build actually reads: the Cargo manifests/lock, the Rust
  # sources, and assets/ (the asset! macro needs them at compile time). Keeping
  # the source minimal means editing docs/flake/nix doesn't bust the build cache.
  # (target/ is already excluded by the flake's git fetcher; this also covers the
  # non-flake/path-fetcher case.)
  cleanedSrc = lib.fileset.toSource {
    root = src;
    fileset = lib.fileset.unions [
      (src + "/Cargo.toml")
      (src + "/Cargo.lock")
      (src + "/src")
      (src + "/assets")
    ];
  };

  version = (craneLib.crateNameFromCargoToml { cargoToml = src + "/Cargo.toml"; }).version;

  commonArgs = {
    src = cleanedSrc;
    inherit version;
    strictDeps = true;
    pname = "tender-db";
  };

  # Native (server-side) dependency artifacts, cached across rebuilds. The wasm
  # client deps are compiled by `dx` during the bundle step below.
  cargoArtifacts = craneLib.buildDepsOnly (
    commonArgs
    // {
      pname = "tender-db-deps";
      cargoExtraArgs = "--features server";
      doCheck = false;
    }
  );

  # ---- Fullstack bundle (server binary + web `public/`) -------------------
  server = craneLib.mkCargoDerivation (
    commonArgs
    // {
      inherit cargoArtifacts;

      # We install the dx bundle ourselves; don't let crane also pack the Cargo
      # target dir into $out as target.tar.zst.
      doInstallCargoArtifacts = false;

      nativeBuildInputs = [
        pkgs.dioxus-cli
        pkgs.binaryen # wasm-opt, used by dx for release optimization
      ];

      # `dx bundle` builds BOTH the wasm client and the native server binary and
      # lays them out as a runnable bundle. `--offline` is required: the sandbox
      # has no network, and crane has already vendored every crate. Pin the out
      # dir so the install step is deterministic. dx writes caches under $HOME.
      buildPhaseCargoCommand = ''
        export DIOXUS_LOG=error
        export HOME=$TMPDIR
        dx bundle --platform web --release --offline --out-dir "$PWD/dx-bundle"
      '';

      # Normalise the layout so $out always has the `server` binary and `public/`
      # at the top level, regardless of whether dx nests them under `web/`.
      installPhaseCommand = ''
        mkdir -p $out
        if [ -d dx-bundle/web ]; then
          cp -r dx-bundle/web/. $out/
        else
          cp -r dx-bundle/. $out/
        fi

        # Expose the server under bin/ so `nix run` works. The binary locates its
        # public/ assets via its own resolved path (/proc/self/exe), so going
        # through this symlink still finds $out/public.
        mkdir -p $out/bin
        ln -s ../server $out/bin/server
      '';

      meta = {
        description = "tender-db fullstack server (Axum + Dioxus WASM client bundle)";
        mainProgram = "server";
        platforms = lib.platforms.unix;
      };
    }
  );
  # Lint gate wired into `nix flake check`. Reuses the server dep cache.
  clippy = craneLib.cargoClippy (
    commonArgs
    // {
      inherit cargoArtifacts;
      cargoClippyExtraArgs = "--features server --all-targets";
    }
  );
in
{
  inherit server clippy;

  devShell = pkgs.mkShell {
    inputsFrom = [ server ];
    packages = [
      toolchain
      pkgs.dioxus-cli
      pkgs.binaryen
      pkgs.cargo-watch
      pkgs.nixfmt
    ];
  };
}
