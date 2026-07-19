# Production runs on Ubuntu via nix, not NixOS

The repo ships a NixOS module and VM smoke test, so a reader would assume the
Hetzner VPS runs NixOS. It deliberately stays Ubuntu: the server bundle is
built by the flake and run under a hand-written systemd unit that mirrors the
module's hardening flags. Reinstalling via nixos-anywhere was considered and
declined (2026-07-19) — the box predates the project, and keeping its
environment familiar outweighs fully-declarative deployment for a one-box
product.

The NixOS module and VM smoke test are kept anyway: the VM test is our only
end-to-end CI check of the bundle in a clean sandbox, and the module is a
distributable artifact for NixOS users of this AGPL project. Do not delete
them as "unused", and do not assume production parity with the module —
production parity lives in the Ubuntu systemd unit.
