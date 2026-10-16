{
  description = "A basic flake for my Bevy Game";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-flake.url = "github:juspay/rust-flake";
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [
        # Optional: use external flake logic, e.g.
        # inputs.foo.flakeModules.default
        inputs.rust-flake.flakeModules.default
        inputs.rust-flake.flakeModules.nixpkgs
      ];
      flake = {
        # Put your original flake attributes here.
      };
      systems = ["x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin"];
      perSystem = {
        config,
        lib,
        pkgs,
        ...
      }: let
        # Pinned to the same nightly as rust-toolchain.toml. `nightly.latest`
        # floated with the rust-overlay input, so `nix flake update` could move
        # the devshell off the toolchain CI uses without touching a single
        # tracked file. Edit both pins together.
        #
        # The PACKAGE builds with this same value (`rust-project.toolchain`
        # below), so there is still one pin here, not two: rust-flake would
        # otherwise resolve its own toolchain from rust-toolchain.toml and the
        # two could drift apart silently.
        rustNightly = pkgs.rust-bin.nightly."2026-07-03".default.override {
          extensions = ["rust-src" "clippy" "rustfmt"];
          targets = ["wasm32-unknown-unknown"];
        };

        # Dev shell `cargo`. It shadows the toolchain cargo on PATH and calls
        # it by store path. The box has 31 GB shared by agent lanes, other
        # Bevy repos and running games; parallel lane builds went 24 GB into
        # swap on 2026-10-15. So a heavy cargo command here takes ONE
        # machine-wide lock (other repos can share the same file) and runs in
        # its own systemd user scope with a memory cap. CI does not use this
        # shell.
        cargoWrapper = pkgs.writeShellScriptBin "cargo" ''
          real=${rustNightly}/bin/cargo
          flock=${pkgs.util-linux}/bin/flock

          # NOVA_CARGO_LOCKED=1 marks a cargo whose parent holds the lock. The
          # wrapper sets it on each cargo it runs under the lock, so a cargo
          # that a test or build script spawns skips the lock and the scope
          # and does not wait for its parent. A `cargo run` program, such as
          # nova_probe_cli, runs after the lock is released and without the
          # mark, so a cargo it spawns takes the lock itself.
          if [ "''${NOVA_CARGO_LOCKED:-}" = 1 ]; then exec "$real" "$@"; fi

          # Find the subcommand after +toolchain and global flags.
          args=("$@")
          i=0
          sub=
          while [ "$i" -lt "''${#args[@]}" ]; do
            case ''${args[$i]} in
              +*) ;;
              -Z | --config | -C | --color | --explain) i=$((i + 1)) ;;
              -*) ;;
              *) sub=''${args[$i]}; break ;;
            esac
            i=$((i + 1))
          done

          case $sub in
            build | b | check | c | test | t | bench | run | r | clippy | doc | d | rustc | install | fix) ;;
            *) exec "$real" "$@" ;;
          esac

          scope=()
          if command -v systemd-run >/dev/null && [ -S "''${XDG_RUNTIME_DIR:-}/bus" ]; then
            scope=(systemd-run --user --scope --quiet --collect -p MemoryHigh=10G -p MemoryMax=12G --)
          else
            echo "cargo: no systemd user bus, running without the memory cap" >&2
          fi

          # The lock lives on fd 9 of this shell. Children get `9>&-` so a
          # daemon they start, such as the sccache server, cannot keep it.
          take_lock() {
            exec 9>>"''${XDG_RUNTIME_DIR:-/tmp}/cargo-build.lock"
            if ! "$flock" -n 9; then
              echo "cargo: waiting for the build lock (another build is running)" >&2
              "$flock" 9
            fi
            ensure_sccache
          }

          # With RUSTC_WRAPPER=sccache, rustc and the linker are children of
          # the sccache server, not of cargo, so the cargo scope does not cap
          # them. Keep the server in its own capped scope. The server is per
          # user and other repos share it, but our lock does not cover their
          # builds, so a stop can fail their compiles. Restart only when the
          # scope is not active, which is once per boot or after the server's
          # idle exit. A scope (not a service) keeps this shell's SCCACHE_*
          # environment.
          ensure_sccache() {
            case ''${RUSTC_WRAPPER:-} in */sccache | sccache) ;; *) return ;; esac
            [ "''${#scope[@]}" -gt 0 ] || return
            [ "$(systemctl --user show nova-sccache.scope -p ActiveState --value 2>/dev/null)" = active ] && return
            "$RUSTC_WRAPPER" --stop-server >/dev/null 2>&1
            systemd-run --user --scope --unit=nova-sccache --quiet --collect \
              -p MemoryHigh=10G -p MemoryMax=12G -- "$RUSTC_WRAPPER" --start-server >/dev/null 2>&1 9>&- ||
              echo "cargo: could not start sccache in nova-sccache.scope, compiles run uncapped" >&2
          }

          if [ "$sub" = run ] || [ "$sub" = r ]; then
            # A running game must not hold the lock. Build the same target
            # under the lock, release the lock, then run unlocked; its compile
            # step is then a no-op. The build gets cargo's options only. The
            # program args start at `--` or at the first positional that is
            # not an option value, as in `cargo run content lint`.
            build=("''${args[@]:0:i}" build)
            j=$((i + 1))
            while [ "$j" -lt "''${#args[@]}" ]; do
              a=''${args[$j]}
              case $a in
                --) break ;;
                -p | --package | --bin | --example | -F | --features | --profile | --target | \
                  --manifest-path | --config | -Z | --color | -j | --jobs | --message-format | \
                  --target-dir | --lockfile-path | -C)
                  build+=("$a")
                  j=$((j + 1))
                  [ "$j" -lt "''${#args[@]}" ] && build+=("''${args[$j]}")
                  ;;
                -*) build+=("$a") ;;
                *) break ;;
              esac
              j=$((j + 1))
            done
            take_lock
            NOVA_CARGO_LOCKED=1 "''${scope[@]}" "$real" "''${build[@]}" 9>&- || exit
            exec 9>&-
            exec "''${scope[@]}" "$real" "$@"
          fi

          take_lock
          NOVA_CARGO_LOCKED=1 "''${scope[@]}" "$real" "$@" 9>&-
        '';

        # mdbook-mermaid 0.17.x implements mdBook's 0.5.0 preprocessor
        # protocol and warns when a newer mdBook invokes it. Keep the producer
        # and preprocessor on the same protocol so doc builds are warning-free.
        mdbook_0_5 = pkgs.rustPlatform.buildRustPackage rec {
          pname = "mdbook";
          version = "0.5.0";
          src = pkgs.fetchFromGitHub {
            owner = "rust-lang";
            repo = "mdBook";
            tag = "v${version}";
            hash = "sha256-KJhCzvwRRK8luSwLN7xNzPanL6Nzlp9+xKGIrSSdyOA=";
          };
          cargoHash = "sha256-cE9oidTX2qb6gaXOfgw3clpAZtpAzhTOfx2EHQjvaLo=";
          meta.mainProgram = "mdbook";
        };

        # The libraries the game needs AT RUN TIME - every one of them opened
        # with `dlopen`, so the linker never records them and a plain binary
        # finds none of them outside a shell that exported them. The devshell
        # exports them as LD_LIBRARY_PATH; the package wraps them onto the
        # binary. ONE list, so a library added for one is never missing from
        # the other.
        gameLibs = with pkgs; [
          udev
          alsa-lib-with-plugins
          vulkan-loader
          libx11
          libxcursor
          libxi
          libxrandr # To use the x11 feature
          libxkbcommon
          wayland # To use the wayland feature
        ];

        # The game binary alone: no assets, no libraries, no desktop entry.
        # Split out so that editing the desktop entry, the icon or the asset
        # wiring costs a second instead of a fat-LTO relink of the whole Bevy
        # graph.
        unwrapped = config.rust-project.crates."nova-protocol".crane.outputs.drv.crate;

        # Read straight out of the source tree rather than through `./assets`,
        # which is a subpath of the flake source and would drag the WHOLE
        # checkout into the package's runtime closure.
        assets = builtins.path {
          path = ./assets;
          name = "nova-protocol-assets";
        };
        credits = builtins.path {
          path = ./credits;
          name = "nova-protocol-credits";
        };
        # The site's brand mark, which is the game's only real icon: the
        # `build/` art is still bevy_game_template's placeholder bird.
        iconSvg = builtins.path {
          path = ./web/src/favicon.svg;
          name = "nova-protocol-icon.svg";
        };

        desktopItem = pkgs.makeDesktopItem {
          name = "nova-protocol";
          desktopName = "Nova Protocol";
          genericName = "Space Shooter";
          comment = "A 3D space shooter game made with Bevy";
          exec = "nova-protocol";
          icon = "nova-protocol";
          terminal = false;
          categories = ["Game" "ActionGame"];
          keywords = ["space" "shooter" "bevy"];
          startupWMClass = "nova-protocol";
        };

        nova-protocol = pkgs.stdenvNoCC.mkDerivation {
          pname = "nova-protocol";
          inherit (unwrapped) version;

          dontUnpack = true;

          nativeBuildInputs = with pkgs; [makeWrapper librsvg];

          installPhase = ''
            runHook preInstall

            mkdir -p $out/share/nova-protocol
            # Symlinks, not copies: 27 MB of art that the build never rewrites,
            # and nix records the store reference through the link.
            ln -s ${assets} $out/share/nova-protocol/assets
            ln -s ${credits} $out/share/nova-protocol/credits

            # BEVY_ASSET_ROOT is the first thing bevy's `get_base_path` reads;
            # without it the reader falls back to the directory of the
            # executable, which in the store holds the binary and nothing else.
            # `--set-default` so a modder can still point a packaged build at a
            # working tree.
            makeWrapper ${unwrapped}/bin/nova-protocol $out/bin/nova-protocol \
              --set-default BEVY_ASSET_ROOT $out/share/nova-protocol \
              --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath gameLibs}

            install -Dm444 ${desktopItem}/share/applications/nova-protocol.desktop \
              -t $out/share/applications

            # A launcher reads the icon by NAME out of the hicolor theme, so
            # the sizes have to exist as files - rofi, wofi and the GTK/KDE
            # menus all resolve `Icon=nova-protocol` this way. The scalable SVG
            # alone is not enough: an icon theme with no `scalable` support
            # falls back to no icon at all.
            install -Dm444 ${iconSvg} \
              $out/share/icons/hicolor/scalable/apps/nova-protocol.svg
            for px in 16 24 32 48 64 128 256 512; do
              dir=$out/share/icons/hicolor/''${px}x''${px}/apps
              mkdir -p $dir
              rsvg-convert -w $px -h $px -o $dir/nova-protocol.png ${iconSvg}
            done

            runHook postInstall
          '';

          meta = {
            description = "A 3D space shooter game made with Bevy";
            homepage = "https://github.com/alexjercan/nova-protocol";
            license = lib.licenses.mit;
            mainProgram = "nova-protocol";
            platforms = lib.platforms.linux;
          };
        };
      in {
        rust-project = {
          toolchain = rustNightly;

          # ONLY the game. The module's default reads `workspace.members` and
          # wires a package, a doc build and an `--all-features -D warnings`
          # clippy check for each of the 24 library crates - 72 derivations,
          # none of them the game, whose Cargo.toml is the workspace ROOT and
          # therefore not a member. `nix flake check` built all of them.
          crates = lib.mkForce {
            "nova-protocol" = {
              path = ./.;
              # The wrapped package below is what `packages` should carry; the
              # bare crate would install a binary that finds no assets.
              autoWire = [];
              crane.args = {
                buildInputs = gameLibs;
                # `dist` is the profile the release workflow ships
                # (.github/workflows/release.yaml), so `nix run` gets the
                # binary a player gets: fat LTO, one codegen unit.
                CARGO_PROFILE = "dist";
                # crane runs `cargo test` in both the deps and the package
                # build. The suite needs a display, a data dir and 27 MB of
                # assets the crane source filter drops, and CI already owns it
                # (.github/workflows/ci.yaml).
                doCheck = false;
              };
              crane.extraBuildArgs.cargoExtraArgs = "--locked -p nova-protocol";
            };
          };
        };

        packages = lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          inherit nova-protocol;
          nova-protocol-unwrapped = unwrapped;
          default = nova-protocol;
        };

        apps = lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          default = {
            type = "app";
            program = lib.getExe nova-protocol;
            meta.description = "Launch Nova Protocol";
          };
        };

        devShells.default = pkgs.mkShell rec {
          nativeBuildInputs = with pkgs; [
            cargoWrapper # first, so it shadows the toolchain cargo
            openssl
            trunk
            wasm-pack
            rustNightly
            clippy
            rust-analyzer
            pkg-config
            llvmPackages.bintools
            nodejs_22 # for the web/ landing site (matches the CI setup-node version)
            samply # sampling profiler for the scenario-dispatch benchmarks
            sccache # RUSTC_WRAPPER: content-hash compile cache shared safely across worktrees
            watchexec # file-watch driver for scripts/serve-mods.sh (the mod portal has no watch mode of its own)
            xvfb-run
            ffmpeg # webm loop encoder (nova_autopilot::loops) + ffprobe for scripts/capture-web-media.sh
            p7zip # Linux fallback for inspecting the macOS release DMG before an itch.io upload
            mdbook_0_5 # developer docs: book.toml at the root, source in docs/, published at /dev/
            mdbook-mermaid # renders the book's ```mermaid fences
            # The sound-design renderers (scripts/gen-*-sfx.py). numpy carries
            # the sample buffers and the FFT; scipy.signal supplies the filter
            # design and resonator banks a layered hit needs. The NOVA OS
            # renderer stays pure stdlib and does not use this.
            (python3.withPackages (ps: with ps; [numpy scipy]))
            butler
          ];

          buildInputs = gameLibs;

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
          RUST_BACKTRACE = 1;

          # sccache: safe fast worktree builds - content-hash compile cache shared
          # across worktrees, each keeping its own target/.
          # sccache requires incremental off.
          RUSTC_WRAPPER = "sccache";
          CARGO_INCREMENTAL = "0";

          # Overrides `jobs = 4` in .cargo/config.toml, which CI reads. The dev
          # box shares 31 GB between agent lanes, other Bevy repos and running
          # games; at 4 per build it went 24 GB into swap (2026-10-15).
          CARGO_BUILD_JOBS = "2";

          RUST_SRC_PATH = "${rustNightly}/lib/rustlib/src/rust/library";
        };
      };
    };
}
