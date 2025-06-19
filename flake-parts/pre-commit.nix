{inputs, ...}: {
  imports = [
    inputs.pre-commit-hooks.flakeModule
  ];
  perSystem = {self', ...}: {
    pre-commit = {
      check.enable = true;

      settings = {
        hooks = {
          statix.enable = true;
          treefmt = {
            enable = true;
            package = self'.packages.treefmt;
          };
        };
        src = ../.;
      };
    };
  };
}
