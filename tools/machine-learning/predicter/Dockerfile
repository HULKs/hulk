FROM tensorflow/tensorflow:latest-gpu AS build-predicter

RUN apt-get update && apt-get install --no-install-recommends --yes \
    build-essential \
    git \
    ninja-build \
    && rm -Rf /var/lib/apt/lists/*

COPY vcpkg-ports/ /vcpkg-ports/
RUN git clone -b "2020.06" https://github.com/microsoft/vcpkg.git && \
    cd vcpkg && \
    ./bootstrap-vcpkg.sh -disableMetrics && \
    ./vcpkg install --recurse --overlay-ports="/vcpkg-ports" compilednn nlohmann-json protobuf cxxopts

COPY predicter/ /predicter/
COPY build-predicter.sh /
RUN ./build-predicter.sh

FROM tensorflow/tensorflow:latest-gpu

COPY --from=build-predicter /predicter/build/predicter /predicter
