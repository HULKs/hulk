FROM ghcr.io/astral-sh/uv:debian
RUN apt-get update && apt-get install -y \
  libegl1 \
  libgl1 \
  libgles2 \
  libglvnd0 \
  libgl1-mesa-dri \
  && rm -rf /var/lib/apt/lists/*
