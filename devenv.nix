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
   pkgs.libxml2
   pkgs.oniguruma
  ];

  enterShell = ''
    export CARGO_HOME="$PWD/.cargo"
    export PATH="$PWD/target/php/bin:$PWD/.cargo/bin/:$PWD/vendor/bin:$PATH"

    # Link the rust stdlib sources to a defined path to ease IDEs integration
    ln -sfT "$RUST_SRC_PATH" "$PWD/.rust-src"

    if [ -z $(which cargo-php) ]; then
      cargo install cargo-php
    fi
  '';
}
