# NixOS VM smoke test: boots a machine running the service via the module and
# checks the HTTP endpoints respond. Consumed by `testers.runNixOSTest`.
self:
{ lib, ... }:
{
  name = "tender-db-vm-smoke";

  nodes.server =
    { ... }:
    {
      imports = [ self.nixosModules.default ];

      services.tender-db = {
        enable = true;
        settings.PORT = 8080;
      };
    };

  testScript = ''
    start_all()

    # Service comes up and binds its port.
    server.wait_for_unit("tender-db.service")
    server.wait_for_open_port(8080)

    # The server renders the app shell at the root.
    server.succeed("curl -sf http://127.0.0.1:8080/ | grep -qi '<html'")

    # A hashed client asset from public/ is actually served. This proves the
    # server resolves its bundle directory (relative to the binary, not the cwd),
    # which the SSR shell above would not catch on its own.
    asset = server.succeed(
        "curl -sf http://127.0.0.1:8080/ | grep -oE '/assets/[^\"]+\\.js' | head -n1"
    ).strip()
    assert asset, "index.html referenced no /assets/*.js bundle"
    server.succeed(f"curl -sf -o /dev/null http://127.0.0.1:8080{asset}")

    # The tender listing server function answers with a JSON array, which also
    # proves the Turso database opened and migrated inside the sandbox.
    server.succeed("curl -sf http://127.0.0.1:8080/api/tenders | grep -q '\\['")
  '';
}
