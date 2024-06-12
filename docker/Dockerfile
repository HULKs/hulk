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
  openssh-server \
  python3 \
  python3-click \
  python3-git \
  rsync \
  zstd

# Expose default SSH port
EXPOSE 22

# Copy host key to image
RUN mkdir -p /root/.ssh && chmod 600 /root/.ssh
COPY id.pub /root/.ssh/authorized_keys

# Set environment variables
RUN echo '\
export RUSTUP_HOME=/usr/local/rustup\n\
export CARGO_HOME=/usr/local/cargo\n\
export PATH=/usr/local/cargo/bin:$PATH' > /root/.bashrc

# Create a dummy git directory
RUN mkdir -p /root/hulk/.git

# Start SSH service
RUN mkdir -p /run/sshd
CMD ["/usr/sbin/sshd", "-D"]
