{inputs, ...}: {
  perSystem = {
    config,
    pkgs,
    system,
    inputs',
    self',
    ...
  }: let
    inherit (self'.packages) rust-toolchain treefmt;
    inherit (self'.legacyPackages) cargoExtraPackages ciPackages;

    devTools = [
      pkgs.bacon
      pkgs.cargo-audit
      pkgs.cargo-udeps
      rust-toolchain
      treefmt
    ];
  in {
    devShells = {
      default = pkgs.mkShell rec {
        packages = devTools ++ cargoExtraPackages;

        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath packages;
        RUST_SRC_PATH = "${self'.packages.rust-toolchain}/lib/rustlib/src/rust/src";

        shellHook = config.pre-commit.installationScript;
      };
    };
  };
}
