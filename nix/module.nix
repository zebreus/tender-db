# NixOS module for tender-db. `import ./nix/module.nix self` yields the module.
#
# Runs the Dioxus fullstack server bundle as a hardened, sandboxed systemd
# service. The bundle ships the Axum server binary plus its `public/` web assets;
# the server reads `IP`/`PORT` from the environment.
self:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.tender-db;
  stateDir = "/var/lib/tender-db";

  # The package install normalises the bundle so the server binary and its
  # `public/` assets sit at the top level of the package.
  serverBin = "${cfg.package}/server";
in
{
  options.services.tender-db = {
    enable = lib.mkEnableOption "the tender-db public-tender server";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      defaultText = lib.literalExpression "tender-db.packages.\${system}.default";
      description = "The fullstack server bundle to run.";
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Open the configured `settings.PORT` in the firewall.";
    };

    hostName = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "tenders.example.org";
      description = ''
        Public host name to serve on. When set, the module enables nginx and adds a
        TLS virtual host (ACME certificate + forced SSL) that reverse-proxies to the
        tender-db server, and opens ports 80/443.

        The operator owns the certificate — set `security.acme.acceptTerms = true`
        and `security.acme.defaults.email`. Leave this `null` to serve plain HTTP and
        terminate TLS yourself.
      '';
    };

    settings = lib.mkOption {
      type = lib.types.submodule {
        # The server reads its config from the environment, so `settings` is a
        # free-form set of env vars passed straight through to the systemd unit.
        freeformType = lib.types.attrsOf lib.types.str;

        options.PORT = lib.mkOption {
          type = lib.types.port;
          default = 8080;
          description = ''
            TCP port the server listens on. Declared explicitly because the
            module needs it for {option}`openFirewall` and for granting the
            privileged-bind capability on ports below 1024.
          '';
        };

        options.IP = lib.mkOption {
          type = lib.types.str;
          default = "0.0.0.0";
          description = "Address the server binds to.";
        };
      };
      default = { };
      example = lib.literalExpression ''
        {
          IP = "0.0.0.0";
          PORT = 8080;
        }
      '';
      description = ''
        Environment variables for the server, passed straight through to the
        systemd unit. Plain values are world-readable in the Nix store, so
        reference secrets via a `*_COMMAND` variable that reads them at runtime.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    # Open the app port when asked, and 80/443 whenever nginx is fronting us.
    networking.firewall.allowedTCPPorts =
      (lib.optionals cfg.openFirewall [ cfg.settings.PORT ])
      ++ (lib.optionals (cfg.hostName != null) [ 80 443 ]);

    # TLS front: standard `services.nginx` + `enableACME`, the operator owning the
    # cert. tender-db itself only ever speaks plain HTTP on `PORT`; nginx proxies
    # to it and terminates TLS.
    services.nginx = lib.mkIf (cfg.hostName != null) {
      enable = true;
      recommendedProxySettings = true;
      recommendedTlsSettings = true;
      recommendedGzipSettings = true;
      virtualHosts.${cfg.hostName} = {
        enableACME = true;
        forceSSL = true;
        locations."/".proxyPass = "http://127.0.0.1:${toString cfg.settings.PORT}";
      };
    };

    systemd.services.tender-db = {
      description = "tender-db public-tender server";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      # The server resolves `public/` relative to its own location in the bundle,
      # so the working directory only needs to be writable state for the Turso
      # database (`tender-db.db`, workdir-relative by default).
      environment = builtins.mapAttrs (_name: toString) cfg.settings;

      serviceConfig = {
        ExecStart = serverBin;
        Restart = "on-failure";

        StateDirectory = "tender-db";
        StateDirectoryMode = "0700";
        WorkingDirectory = stateDir;

        # Run as a transient, unprivileged user.
        DynamicUser = true;

        # Sandbox: writable access limited to the state directory.
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        PrivateDevices = true;
        ProtectProc = "invisible";
        ProcSubset = "pid";
        UMask = "0077";

        NoNewPrivileges = true;
        RestrictNamespaces = true;
        LockPersonality = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        ProtectControlGroups = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectKernelLogs = true;
        ProtectClock = true;
        ProtectHostname = true;
        RemoveIPC = true;

        # AF_UNIX is needed for hostname resolution: glibc NSS talks to NixOS's
        # nscd/nsncd over a unix socket, so outbound HTTPS from the (future)
        # tender importer would otherwise fail at getaddrinfo.
        RestrictAddressFamilies = [
          "AF_UNIX"
          "AF_INET"
          "AF_INET6"
        ];

        CapabilityBoundingSet = lib.optionals (cfg.settings.PORT < 1024) [ "CAP_NET_BIND_SERVICE" ];
        AmbientCapabilities = lib.optionals (cfg.settings.PORT < 1024) [ "CAP_NET_BIND_SERVICE" ];

        SystemCallArchitectures = "native";
        SystemCallFilter = [
          "@system-service"
          "~@privileged"
          "~@resources"
        ];
      };
    };
  };
}
