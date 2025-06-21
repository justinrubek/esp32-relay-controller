{
  lib,
  stdenv,
  fetchurl,
  zlib,
}: let
  rpath = "${zlib}/lib:$out/lib";
in
  stdenv.mkDerivation rec {
    pname = "rust-src-esp";
    version = "1.86.0.0";

    src = fetchurl {
      url = "https://github.com/esp-rs/rust-build/releases/download/v${version}/rust-src-${version}.tar.xz";
      sha256 = "sha256-EPoxNiYUk6XZfU886bmLruXMWCiXEf5vJCSY/09lspo=";
    };

    installPhase = ''
      patchShebangs install.sh
      CFG_DISABLE_LDCONFIG=1 ./install.sh --prefix=$out

      rm $out/lib/rustlib/{components,install.log,manifest-*,rust-installer-version,uninstall.sh} || true

      ${lib.optionalString stdenv.isLinux ''
        if [ -d $out/bin ]; then
          for file in $(find $out/bin -type f); do
            if isELF "$file"; then
              patchelf \
                --set-interpreter ${stdenv.cc.bintools.dynamicLinker} \
                --set-rpath ${rpath} \
                "$file" || true
            fi
          done
        fi

        if [ -d $out/lib ]; then
          for file in $(find $out/lib -type f); do
            if isELF "$file"; then
              patchelf --set-rpath ${rpath} "$file" || true
            fi
          done
        fi

        if [ -d $out/libexec ]; then
          for file in $(find $out/libexec -type f); do
            if isELF "$file"; then
              patchelf \
                --set-interpreter ${stdenv.cc.bintools.dynamicLinker} \
                --set-rpath ${rpath} \
                "$file" || true
            fi
          done
        fi

        for file in $(find $out/lib/rustlib/*/bin -type f); do
          if isELF "$file"; then
            patchelf \
              --set-interpreter ${stdenv.cc.bintools.dynamicLinker} \
              --set-rpath ${stdenv.cc.cc.lib}/lib:${rpath} \
              "$file" || true
          fi
        done
      ''}
    '';
    dontStrip = true;
  }
