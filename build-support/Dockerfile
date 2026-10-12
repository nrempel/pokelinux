FROM debian:trixie-slim
RUN dpkg --add-architecture arm64 && apt-get update && \
    apt-get install -y --no-install-recommends \
      build-essential gcc-aarch64-linux-gnu g++-aarch64-linux-gnu \
      autoconf automake libtool pkg-config cmake python3 python3-pyelftools \
      qemu-user clang-format ca-certificates patch \
      libegl-dev:arm64 libgles-dev:arm64 libdrm-dev:arm64 libgbm-dev:arm64 \
      libwayland-dev:arm64 libffi-dev:arm64 libexpat1-dev:arm64 \
      libegl-mesa0:arm64 libgl1-mesa-dri:arm64 && \
    rm -rf /var/lib/apt/lists/*
ENV PKG_CONFIG_LIBDIR=/usr/lib/aarch64-linux-gnu/pkgconfig:/usr/share/pkgconfig
WORKDIR /workspace
