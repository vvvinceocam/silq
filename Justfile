default:
  @just --list

default_php_version := "8.2.11"

build-php version=default_php_version:
    #!/usr/bin/env bash
    set -euxo pipefail

    project_dir=$(pwd)

    if [ ! -d php-src ]; then
      git clone https://github.com/php/php-src.git;
      cd php-src
      git checkout "php-{{version}}"
    else
      cd php-src
      git reset --hard
      git fetch --tags
      git checkout "php-{{version}}"
    fi

    ./buildconf --force
    PREFIX="${project_dir}/php-src/build"
    ./configure --prefix="${PREFIX}" \
      --disable-cgi --without-pdo --without-pdo-sqlite --without-sqlite3 \
      --enable-debug --enable-mbstring --enable-xml --with-libxml --with-curl
    make -j "$(nproc)"
    make install

build:
    cargo build

run-tests: run-unit-tests run-integration-tests

run-unit-tests: build
  cargo test
  php -d extension=./target/debug/libsilq.so vendor/bin/pest --filter Unit

run-integration-tests: build
  docker-compose -f tests/docker-compose.yaml up -d
  sleep 1 # work around to let the service be ready, TODO: use docker's `healthcheck`
  php -d extension=./target/debug/libsilq.so vendor/bin/pest --filter Feature
  docker-compose -f tests/docker-compose.yaml down

run-benchmarks: build
  docker-compose -f tests/docker-compose.yaml up -d
  sleep 1 # work around to let the service be ready, TODO: use docker's `healthcheck`
  php -d extension=./target/debug/libsilq.so ./vendor/bin/phpbench run tests/Benchmark --report=default
  docker-compose -f tests/docker-compose.yaml down
