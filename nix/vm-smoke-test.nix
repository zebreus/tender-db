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

    # The public API is merged beside the dioxus router and reads the canonical
    # layer: a fresh database answers with an empty page, not an error. This is
    # what proves the reader connections opened and the `v_*` views exist.
    server.succeed(
        "curl -sf -H 'Accept: application/json' http://127.0.0.1:8080/v1/tenders "
        "| grep -q '\"items\":\\[\\]'"
    )

    # `/health` is what deploy.sh probes: it must report ok, name the revision
    # the binary was built from, and prove the database answered.
    health = server.succeed("curl -sf http://127.0.0.1:8080/health")
    assert '"ok":true' in health, f"/health did not report ok: {health}"
    assert '"database":"ok"' in health, f"/health did not reach the database: {health}"
    assert '"rev"' in health, f"/health did not name a revision: {health}"

    # AGPL section 13: the running service links its own source at the API root.
    server.succeed("curl -sf http://127.0.0.1:8080/v1 | grep -q source_offer")
  '';
}
