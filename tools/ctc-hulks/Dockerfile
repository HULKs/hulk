FROM ubuntu:18.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update
RUN apt-get install -y sudo build-essential automake autoconf gperf bison \
  flex texinfo libtool libtool-bin gawk libncurses5-dev unzip cmake \
  libexpat-dev python2.7-dev nasm help2man ninja-build git lzip wget vim zsh \
  libdrm-dev pkg-config pbzip2
RUN groupadd -r -g 1000 ctc
RUN useradd -r -u 1000 -g ctc ctc
RUN usermod -aG sudo ctc
RUN /usr/bin/chsh -s /usr/bin/zsh ctc
USER ctc
