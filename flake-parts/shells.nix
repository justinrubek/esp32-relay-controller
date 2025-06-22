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
    inherit (self'.legacyPackages) cargoExtraPackages;

    devTools = [
      pkgs.bacon
      pkgs.cargo-audit
      pkgs.cargo-udeps
      pkgs.espflash
      pkgs.ldproxy
      treefmt
    ];
  in {
    packages.t = inputs'.esp-flake.packages.rust-xtensa;
    devShells = {
      default = pkgs.mkShell rec {
        packages = devTools ++ cargoExtraPackages;
        buildInputs = [
          inputs'.esp-flake.packages.esp-idf-full
          inputs'.esp-flake.packages.llvm-xtensa
          inputs'.esp-flake.packages.llvm-xtensa-lib
          inputs'.esp-flake.packages.espflash
          inputs'.esp-flake.packages.ldproxy
          pkgs.git
          pkgs.wget
          pkgs.gnumake
          pkgs.flex
          pkgs.bison
          pkgs.gperf
          pkgs.pkg-config
          pkgs.cargo-generate
          pkgs.python3
          pkgs.python3Packages.pip
          pkgs.python3Packages.virtualenv
          pkgs.cmake
          pkgs.ninja
          pkgs.ncurses5
          pkgs.platformio-core
          rust-toolchain
        ];

        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath packages;
        RUST_SRC_PATH = "${rust-toolchain}/lib/rustlib/src/rust/src";

        shellHook = ''
          ${config.pre-commit.installationScript}
          export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath [pkgs.libxml2 pkgs.zlib pkgs.stdenv.cc.cc.lib]}
          export ESP_IDF_VERSION=${inputs'.esp-flake.packages.esp-idf-full.version}
          export LIBCLANG_PATH=${inputs'.esp-flake.packages.llvm-xtensa-lib}/lib
          export RUSTFLAGS="--cfg espidf_time64"
        '';
      };
    };
  };
}
