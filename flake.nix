{
  description = "Messaging automation for small businesses: WhatsApp, Instagram DM, lead capture";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane, git-hooks }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "clippy" "rustfmt" ];
          targets = [ "wasm32-unknown-unknown" ];
        };
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Include assets/ alongside standard Cargo sources
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            (craneLib.filterCargoSources path type)
            || (builtins.match ".*assets/.*" path != null);
        };

        # Common args for all crane builds
        commonArgs = {
          inherit src;
          strictDeps = true;
        };

        # Build dependencies separately for caching
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        pre-commit-check = git-hooks.lib.${system}.run {
          src = ./.;
          hooks = {
            check-json.enable = true;
            check-merge-conflicts.enable = true;
            check-toml.enable = true;
            check-yaml.enable = true;
            detect-private-keys.enable = true;
            end-of-file-fixer.enable = true;
            mixed-line-endings.enable = true;
            trim-trailing-whitespace.enable = true;
            nixpkgs-fmt.enable = true;
            rustfmt = {
              enable = true;
              packageOverrides.cargo = rustToolchain;
              packageOverrides.rustfmt = rustToolchain;
            };
            # clippy runs via crane checks (needs network for deps);
          };
        };
      in
      {
        checks = {
          inherit pre-commit-check;

          # Run cargo test
          tests = craneLib.cargoTest (commonArgs // {
            inherit cargoArtifacts;
          });

          # Run clippy with all warnings as errors
          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- -D warnings";
          });

          # Check formatting
          fmt = craneLib.cargoFmt {
            inherit src;
          };
        };

        # `nix run .#dev` — local dev server with the management-panel
        # bypass active: applies migrations against the local D1, sets
        # MANAGE_BYPASS_EMAIL so /manage skips Cloudflare Access JWT
        # verification, and stubs the AI bindings (see `crate::dev_bypass`).
        # Production deploys never set MANAGE_BYPASS_EMAIL and always
        # set CF_ACCESS_AUD, so the bypass cannot activate there.
        apps.dev = {
          type = "app";
          program = toString (pkgs.writeShellScript "concierge-dev" ''
            export PATH=${pkgs.lib.makeBinPath [ pkgs.wrangler pkgs.nodejs_22 ]}:$PATH
            cd ${toString ./.}
            exec ${pkgs.nodejs_22}/bin/node scripts/test-server.mjs "$@"
          '');
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            wrangler
            worker-build
            wasm-pack
            binaryen
            nodejs_22
            nodePackages.npm
            # Headless Chromium for `npm run screenshots` (drives the docs
            # gallery in doc/screenshots/ and gives us visual-regression
            # checks against the live welcome / login templates).
            playwright-driver.browsers
          ];
          shellHook = ''
            ${pre-commit-check.shellHook}
            export PLAYWRIGHT_BROWSERS_PATH=${pkgs.playwright-driver.browsers}
            export PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS=true
            # `dev` runs the local server with the management bypass +
            # migrations applied — same shim Playwright uses, but for a
            # human pointing a browser at http://localhost:8787/manage.
            dev() {
              node scripts/test-server.mjs "$@"
            }
            echo "Concierge Worker dev environment"
            echo "  dev                 - Local server + /manage bypass + migrations (run from this shell)"
            echo "  nix run .#dev       - Same as above, runnable from any shell"
            echo "  wrangler dev        - Plain dev server (no bypass; /manage will 403)"
            echo "  wrangler deploy     - Deploy to Cloudflare"
            echo "  npm test            - Run Playwright browser tests"
            echo "  npm run screenshots - Regenerate docs gallery PNGs"
            echo "  nix flake check     - Run CI checks"
          '';
        };
      }
    );
}
