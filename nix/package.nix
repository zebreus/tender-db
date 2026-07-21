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

  # Only the files the build actually reads: the workspace manifests/lock and
  # the crates (sources + each crate's assets for the asset! macro). Keeping
  # the source minimal means editing docs/flake/nix doesn't bust the build cache.
  # (target/ is already excluded by the flake's git fetcher; this also covers the
  # non-flake/path-fetcher case.)
  cleanedSrc = lib.fileset.toSource {
    root = src;
    fileset = lib.fileset.unions [
      (src + "/Cargo.toml")
      (src + "/Cargo.lock")
      (src + "/crates")
    ];
  };

  # The root manifest is a virtual workspace; all member versions inherit from it.
  version = (lib.importTOML (src + "/Cargo.toml")).workspace.package.version;

  commonArgs = {
    src = cleanedSrc;
    inherit version;
    strictDeps = true;
    pname = "tender-db";
  };

  # Native (server-side) dependency artifacts under the default `release` profile,
  # for the clippy lint gate below (plain `cargo clippy`, host target).
  cargoArtifacts = craneLib.buildDepsOnly (
    commonArgs
    // {
      pname = "tender-db-deps";
      cargoExtraArgs = "-p tender-db --features server";
      doCheck = false;
    }
  );

  # Dependency cache for the fullstack bundle. `dx` compiles the graph under two
  # ad-hoc profiles/targets a vanilla crane never populates — the server at
  # `target/<host>/server-release/` and the wasm client at
  # `target/wasm32-unknown-unknown/wasm-release/`. The plain `cargoArtifacts`
  # above lands in `target/release/`, which dx never reads, so before this every
  # deploy cold-built the whole ~1300-crate native + wasm graph (~20 min), even
  # for a one-line server change.
  #
  # We prime the cache with `dx` itself (against crane's DUMMY workspace sources)
  # so the fingerprints match by construction — matching dx's ad-hoc profiles with
  # plain cargo does not reproduce them. Two separate `dx build`s rather than one
  # `dx bundle`: the wasm-bindgen step fails on the contentless dummy app, and a
  # single bundle would abort the server build with it; run apart, each compiles
  # its whole dependency graph (all we snapshot) before that cosmetic failure.
  # The real bundle then recompiles only the four workspace crates (~seconds).
  #
  # Coupled to dx's profile names/flags: a dx upgrade that changes them just makes
  # the cache miss and rebuild — slower, never wrong.
  bundleDeps = craneLib.buildDepsOnly (
    commonArgs
    // {
      pname = "tender-db-bundle-deps";
      doCheck = false;
      nativeBuildInputs = [
        pkgs.dioxus-cli
        pkgs.binaryen
      ];
      buildPhaseCargoCommand = ''
        export DIOXUS_LOG=error
        export HOME=$TMPDIR
        dx build --package tender-db --platform server --release --offline
        dx build --package tender-db --platform web --release --offline || true

        # Drop the four DUMMY workspace crates from the cache so the real bundle
        # recompiles them from scratch. dx compiles the app crate as BOTH a lib and
        # a bin and passes the lib to the bin as `--extern`; leaving the stub lib in
        # the cache links the real bin against an empty client (a silent wrong-output
        # bug, or an E0599 on the mismatched component props). Only the ~1300
        # third-party dep artifacts stay cached. No registry crate shares these names.
        for c in tender_db model store ingest; do
          find target -type f \( -name "lib$c-*" -o -name "$c-*" \) -delete
          find target -type d -path '*/.fingerprint/*' -name "$c-*" -exec rm -rf {} +
        done
      '';
    }
  );

  # ---- Fullstack bundle (server binary + web `public/`) -------------------
  server = craneLib.mkCargoDerivation (
    commonArgs
    // {
      cargoArtifacts = bundleDeps;

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
        dx bundle --package tender-db --platform web --release --offline --out-dir "$PWD/dx-bundle"
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
      cargoClippyExtraArgs = "--workspace --all-targets --features tender-db/server";
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
