{ pkgs, ... }:

{
  languages.rust = {
    enable = true;
    channel = "stable";
  };

  pre-commit.hooks = {
    clippy.enable = true;
    rustfmt.enable = true;
  };

  packages = [
   pkgs.git
   pkgs.just
   pkgs.php82.packages.composer

   # deps to build PHP
   pkgs.gnumake
   pkgs.bison
   pkgs.re2c
   pkgs.autoconf
   pkgs.libxml2
   pkgs.oniguruma
   pkgs.curl

   # required by cargo-audit
   pkgs.openssl
  ];

  scripts = {
    setup.exec = ''
      if [ ! -d php-src ]; then
        just build-php
      fi

      if [ -z $(which cargo-php) ]; then
        cargo install cargo-php
      fi

      if [ -z $(which cargo-audit) ]; then
        cargo install cargo-audit
      fi
    '';
  };

  enterShell = ''
    export CARGO_HOME="$PWD/.cargo"
    export PATH="$PWD/php-src/build/bin:$PWD/.cargo/bin/:$PWD/vendor/bin:$PATH"

    # Link the rust stdlib sources to a defined path to ease IDEs integration
    ln -sfT "$RUST_SRC_PATH" "$PWD/.rust-src"
  '';
}
