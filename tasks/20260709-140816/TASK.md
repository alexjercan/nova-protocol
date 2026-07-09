# Reconcile CI test story: AGENTS.md defers tests to CI but no PR workflow exists

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: v0.4.0,chore,ci

AGENTS.md ("Build, run, test") instructs agents to skip local cargo test and
clippy because "both run in CI on every PR". Review R1.2 of task
20260709-131502 checked: .github/workflows contains only deploy-page.yaml
(workflow_dispatch) and release.yaml (tag/dispatch). No in-repo workflow runs
cargo test or clippy on PRs, so under the current setup the full suite runs
nowhere unless it happens outside this repo.

Resolve one way or the other:

## Steps

- [x] Confirm with the user where (or whether) PR checks actually run today
      (external CI? pre-push habit? nowhere?). Resolved: user directed to add
      in-repo PR checks (they ran nowhere before). Borrowed the shape from
      ~/personal/bevy-common-systems/.github/workflows/ci.yml.
- [x] If checks should exist: add .github/workflows/ci.yaml running
      `cargo test --workspace` and `cargo clippy --all-targets` on pull_request
      (and push to master), with the system libs the dev shell provides (see
      flake/nix docs; Xvfb needed for the headless example smoke tests).
- [x] If checks intentionally live elsewhere: document where in AGENTS.md so
      the "CI is the source of truth" claim is verifiable. (Pointed AGENTS.md
      at .github/workflows/ci.yaml instead - checks now live in-repo.)
- [x] Keep the AGENTS.md skip-local-tests instruction consistent with reality.

## Notes

- Found during review of 20260709-131502 (see its REVIEW.md R1.2).
- The workspace suite takes ~1-2 minutes locally; the examples smoke test
  dominates and needs a display (Xvfb) plus the vulkan/alsa/udev libs.
