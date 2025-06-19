{inputs, ...}: {
  imports = [
    inputs.bomper.flakeModules.bomper
  ];
  perSystem = {
    config,
    pkgs,
    self',
    ...
  }: {
    bomper = {
      enable = true;
      configuration = ''
        (
            cargo: Some(Autodetect),
            authors: Some({
                "Justin Rubek": "justinrubek"
            }),
        )
      '';
    };
    devShells.ci = pkgs.mkShell rec {
      packages = [config.bomper.wrappedBomper];
      LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath packages;
    };
  };
}
