FROM node:latest

RUN apt-get update && apt-get install --no-install-recommends --yes \
  git-lfs \
  && rm --recursive --force /var/lib/apt/lists/*

RUN useradd --no-log-init --create-home --home-dir /ci --system ci
WORKDIR /ci
USER ci:ci

ARG RUNNER_VERSION
RUN mkdir actions-runner && cd actions-runner && \
  wget --no-verbose https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz && \
  tar --extract --gzip --file actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz && \
  rm actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz
VOLUME /ci/actions-runner

WORKDIR /ci/actions-runner
CMD ["/ci/actions-runner/bin/runsvc.sh"]
