{
  description = "Starship-jj shows jujutsu-vcs status for the starship prompt";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    fleet.url = "github:averagechris/fleet/e31a02573d79dfeb2496fec6c21cf74a0ece4d79";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    systems.url = "github:nix-systems/default";
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.systems.follows = "systems";
    };
  };
  outputs =
    {
      self,
      flake-utils,
      fenix,
      fleet,
      nixpkgs,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages."${system}";
        lib = pkgs.lib;
        rustBuildToolchain = fenix.packages.${system}.stable.minimalToolchain;
        rustDevToolchain = fenix.packages.${system}.stable.toolchain;
        rootCargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        rootPackage = rootCargoToml.workspace.package or rootCargoToml.package or null;
        longDescription =
          if rootPackage ? readme then builtins.readFile (./. + ("/" + rootPackage.readme)) else null;
        homepage = rootPackage.homePage or rootPackage.repository or null;
        license = rootPackage.license or null;
        rustDependencyTools = with pkgs; [
          cargo-audit
          cargo-deny
          cargo-license
          cargo-machete
          cargo-outdated
          cargo-udeps
        ];

        rustPackage =
          bin-dir: features:
          with builtins;
          let
            cargoToml = fromTOML (readFile (bin-dir + "/Cargo.toml"));
            pname = cargoToml.package.name;
            inherit (cargoToml.package) version;
            description = cargoToml.package.description or null;
          in
          with pkgs;
          (makeRustPlatform {
            cargo = rustBuildToolchain;
            rustc = rustBuildToolchain;
          }).buildRustPackage
            {
              inherit pname version;
              src = lib.cleanSource ./.;
              cargoLock.lockFile = ./Cargo.lock;
              buildFeatures = features;
              buildInputs = [ openssl ];
              nativeBuildInputs = [ pkg-config ];
              cargoBuildFlags = [
                "-p"
                cargoToml.package.name
              ];
              meta = lib.attrsets.filterAttrs (k: v: v != null) {
                inherit
                  homepage
                  license
                  description
                  longDescription
                  ;
                mainProgram = pname;
              };
            };
        starship-jj-pkg = rustPackage ./. [ ];
        starship-jj-dotenv-pkg = rustPackage ./. [ "dotenv" ];

        # --- standard fleet release interface -------------------------------
        # Release plumbing (prepare-release, release-tag, release,
        # release-artifact) and ci-clippy/ci-test come from the shared fleet
        # preset; ci-fmt stays local because it adds the repo's nixfmt gate.
        fleetApps = fleet.lib.fleet.presets.rust {
          inherit pkgs self;
          srhtPackage = fleet.packages.${system}.srht;
          pname = "starship-jj";
          releaseBackend = "github";
          releaseValidationApps = [ "ci-release-contract" ];
          ciFmt = ci-fmt;
        };
        fleetReleaseArtifact = fleetApps.releaseArtifact system;
        cliProgram = "starship-jj";
        fetchUpstreamScript = ''
          # Refresh upstream refs in both git and jj. For jj, Git's
          # refs/remotes/upstream/<branch> is exposed as the remote bookmark
          # <branch>@upstream.
          exec jj git fetch --remote upstream "$@"
        '';
        ciFmtScript = ''
          cargo fmt --all --check
          nixfmt --check flake.nix
        '';
        mkRepoScript =
          {
            name,
            runtimeInputs ? [ ],
            text,
          }:
          pkgs.writeShellApplication {
            inherit name runtimeInputs;
            inherit text;
          };
        fetch-upstream = mkRepoScript {
          name = "fetch-upstream";
          text = fetchUpstreamScript;
          runtimeInputs = with pkgs; [
            jujutsu
          ];
        };
        ci-fmt = mkRepoScript {
          name = "ci-fmt";
          text = ciFmtScript;
          runtimeInputs = with pkgs; [
            nixfmt
            rustDevToolchain
          ];
        };
        ci-release-contract = mkRepoScript {
          name = "ci-release-contract";
          runtimeInputs = with pkgs; [ gnugrep ];
          text = ''
            help="$(${fleetApps.apps.release.program} --help)"
            printf '%s\n' "$help" | grep -Fqx \
              'usage: release --version X.Y.Z [--check] [--allow-downgrade]'
            printf '%s\n' "$help" | grep -Fq -- \
              '--check  nonmutating ref/version preflight only; does not run validation or build artifacts'
            grep -Fqx '    nix run .#release -- --version X.Y.Z --check' AGENTS.md
            grep -Fqx '    nix run .#release -- --version X.Y.Z' AGENTS.md
            if printf '%s\n' "$help" | grep -Eq -- '--(submit-linux-build|skip-(validate|tag|artifact|pages)|publish-pages)'; then
              printf '%s\n' 'release help exposes an obsolete SourceHut or bypass flag' >&2
              exit 1
            fi
          '';
        };
        repo-scripts = pkgs.symlinkJoin {
          name = "starship-jj-scripts";
          paths = [
            ci-fmt
            fetch-upstream
          ];
        };
        fmt-check =
          pkgs.runCommand "starship-jj-fmt-check"
            {
              nativeBuildInputs = [ ci-fmt ];
              src = lib.cleanSource ./.;
            }
            ''
              export HOME="$TMPDIR"
              cp -R "$src" source
              chmod -R +w source
              cd source
              ci-fmt
              mkdir -p "$out"
            '';
        # `nix fmt` invokes the formatter app without path arguments; nixfmt
        # treats no arguments as "format stdin", so default it to the flake.
        nix-formatter = pkgs.writeShellApplication {
          name = "nixfmt";
          runtimeInputs = with pkgs; [
            nixfmt
          ];
          text = ''
            if [[ $# -eq 0 ]]; then
              exec nixfmt flake.nix
            fi

            exec nixfmt "$@"
          '';
        };
      in
      {
        formatter = nix-formatter;

        packages = {
          default = starship-jj-pkg;
          starship-jj = starship-jj-pkg;
          # Variant with dotenv feature enabled (opt-in for dev/testing)
          starship-jj-dotenv = starship-jj-dotenv-pkg;
          ci-fmt = ci-fmt;
          ci-release-contract = ci-release-contract;
          fetch-upstream = fetch-upstream;
          scripts = repo-scripts;
        }
        // lib.optionalAttrs (fleetReleaseArtifact != null) {
          "release-artifact" = fleetReleaseArtifact;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = starship-jj-pkg;
          exePath = "/bin/${cliProgram}";
        };
        apps.starship-jj = flake-utils.lib.mkApp {
          drv = starship-jj-pkg;
          exePath = "/bin/${cliProgram}";
        };
        apps.ci-fmt = flake-utils.lib.mkApp {
          drv = ci-fmt;
        };
        apps.ci-release-contract = flake-utils.lib.mkApp {
          drv = ci-release-contract;
        };
        apps.fetch-upstream = flake-utils.lib.mkApp {
          drv = fetch-upstream;
        };
        apps.ci-clippy = fleetApps.apps.ci-clippy;
        apps.static-checks = fleetApps.apps.static-checks;
        apps.ci-test = fleetApps.apps.ci-test;
        apps.prepare-release = fleetApps.apps.prepare-release;
        apps.release = fleetApps.apps.release;
        apps.release-tag = fleetApps.apps.release-tag;

        checks = {
          build = starship-jj-pkg;
          fmt = fmt-check;
          release-contract = pkgs.runCommand "starship-jj-release-contract" { } ''
            cd ${self}
            ${ci-release-contract}/bin/ci-release-contract
            touch "$out"
          '';
        }
        // lib.optionalAttrs (fleetReleaseArtifact != null) {
          "release-artifact" = fleetReleaseArtifact;
        };

        devShells.default = pkgs.mkShell {
          packages =
            with pkgs;
            [
              rust-analyzer
              rustDevToolchain
              just
              repo-scripts
            ]
            ++ rustDependencyTools;
          inputsFrom = [ self.packages."${system}".starship-jj ];
        };
        # Dev shell that prebuilds the dotenv-enabled variant; you can still
        # toggle at cargo time with `--features dotenv`.
        devShells.dotenv = pkgs.mkShell {
          packages =
            with pkgs;
            [
              rust-analyzer
              rustDevToolchain
              just
              repo-scripts
            ]
            ++ rustDependencyTools;
          inputsFrom = [ self.packages."${system}".starship-jj-dotenv ];
        };
      }
    )
    // {
      overlays.default = final: prev: {
        inherit (self.packages."${prev.stdenv.hostPlatform.system}") starship-jj;
      };
    };
}
