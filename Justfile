default:
  @just --list

default_php_version := "8.2.8"

build-php version=default_php_version:
    #!/usr/bin/env bash
    set -euxo pipefail

    project_dir=$(pwd)

    if [ ! -d php-src ]; then git clone https://github.com/php/php-src.git; fi

    cd php-src
    git checkout "php-{{version}}"

    ./buildconf --force
    PREFIX="${project_dir}/target/php"
    ./configure --prefix="${PREFIX}" \
      --disable-cgi --without-pdo --without-pdo-sqlite --without-sqlite3 \
      --enable-debug --enable-mbstring --enable-xml --with-libxml
    make -j "$(nproc)"
    make install

build:
    cargo build

run-tests: run-unit-tests run-integration-tests

run-unit-tests: build
  cargo test
  pest --filter Unit

run-integration-tests: build
  pest --filter Feature
