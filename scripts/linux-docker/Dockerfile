# Stesso ambiente del job "linux" della CI (ubuntu-22.04 + .github/actions/setup-build).
FROM ubuntu:22.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y curl ca-certificates git xz-utils file \
    build-essential libwebkit2gtk-4.1-dev libxdo-dev libssl-dev \
    libayatana-appindicator3-dev librsvg2-dev && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && apt-get install -y nodejs
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --component rustfmt,clippy
ENV PATH=/root/.cargo/bin:$PATH
WORKDIR /src
