{
  perSystem = {inputs', ...}: {
    packages = {
      rust-toolchain = inputs'.esp-flake.packages.rust-xtensa;
    };
  };
}
