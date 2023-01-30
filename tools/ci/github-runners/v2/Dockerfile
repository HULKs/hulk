FROM rust:latest

RUN apt-get update && apt-get install --no-install-recommends --yes \
  clang \
  cmake \
  git-lfs \
  libasound2-dev \
  libdbus-1-dev \
  libhdf5-dev \
  libluajit-5.1-dev \
  libsystemd-dev \
  libudev-dev \
  ninja-build \
  python3 \
  python3-click \
  python3-git \
  rsync \
  zstd \
  && rm --recursive --force /var/lib/apt/lists/*

RUN rustup component add clippy rustfmt

RUN useradd --no-log-init --create-home --home-dir /ci --system ci
WORKDIR /ci
USER ci:ci

ARG RUNNER_VERSION
RUN mkdir actions-runner && cd actions-runner && \
  wget --no-verbose https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz && \
  tar --extract --gzip --file actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz && \
  rm actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz
VOLUME /ci/actions-runner

ARG SDK_VERSION
COPY HULKs-OS-toolchain-${SDK_VERSION}.sh /
RUN /HULKs-OS-toolchain-${SDK_VERSION}.sh -y -d /ci/.naosdk/${SDK_VERSION}

WORKDIR /ci/actions-runner
CMD ["/ci/actions-runner/bin/runsvc.sh"]
