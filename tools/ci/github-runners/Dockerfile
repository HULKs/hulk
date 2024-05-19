FROM rust:latest

RUN apt-get update && apt-get install --no-install-recommends --yes \
  clang \
  cmake \
  git-lfs \
  libasound2-dev \
  libdbus-1-dev \
  libhdf5-dev \
  libluajit-5.1-dev \
  libopusfile-dev \
  libsystemd-dev \
  libudev-dev \
  lua-inspect \
  mkdocs \
  mkdocs-material \
  ninja-build \
  python3 \
  python3-click \
  python3-git \
  rsync \
  zstd \
  && rm --recursive --force /var/lib/apt/lists/*

RUN rustup component add clippy rustfmt
