{
  description = "agrf — stream of numbers to braille unicode graphs";

  # Indirect ref: on a machine whose flake registry already has nixpkgs
  # realised (e.g. the author's), this reuses that store path. Consumers get
  # whatever the lock pins — override with inputs.agrf.inputs.nixpkgs.follows.
  inputs.nixpkgs.url = "flake:nixpkgs";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems
        (system: f nixpkgs.legacyPackages.${system});
    in {
      packages = forAll (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "agrf";
          # Read straight from Cargo.toml so the two never drift apart.
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
          src = self;
          # Cargo.lock is committed, so deps resolve straight from it — no
          # cargoHash to recompute on every dependency bump.
          cargoLock.lockFile = ./Cargo.lock;

          postInstall = ''
            install -Dm644 man/man1/agrf.1 $out/share/man/man1/agrf.1
          '';
        };
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            rustc
            cargo
            clippy
            rustfmt
            rust-analyzer
          ];
        };
      });
    };
}
