FROM debian:bullseye

RUN apt-get update && apt-get install --no-install-recommends --yes \
  build-essential \
  ca-certificates \
  clang \
  cmake \
  curl \
  file \
  git-lfs \
  gzip \
  libasound2-dev \
  libdbus-1-dev \
  libhdf5-dev \
  libluajit-5.1-dev \
  libsystemd-dev \
  ninja-build \
  python3 \
  python3-click \
  python3-git \
  rsync \
  wget \
  xz-utils \
  && rm --recursive --force /var/lib/apt/lists/*

COPY installer.sh /
RUN /installer.sh -y -d /opt/nao

RUN useradd --no-log-init --create-home --home-dir /ci --system ci
WORKDIR /ci
USER ci:ci

# We need to do this as the ci user and modify PATH to actually use the stuff
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y
ENV PATH=/ci/.cargo/bin:$PATH

RUN rustup component add clippy rustfmt

ARG RUNNER_VERSION
RUN mkdir actions-runner && cd actions-runner && \
  wget --no-verbose https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz && \
  tar --extract --gzip --file actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz && \
  rm actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz
VOLUME /ci/actions-runner

WORKDIR /ci/actions-runner
CMD ["/ci/actions-runner/bin/runsvc.sh"]
