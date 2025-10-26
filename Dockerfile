FROM rust AS build
WORKDIR /usr/src/mee6
COPY . .
RUN apt-get update
RUN apt-get install -y --no-install-recommends \
    curl \
    automake \
    cmake \
    ffmpeg \
    clang \
    libopus0 libopus-dev \
    opus-tools
RUN cargo install --path .


FROM ubuntu:24.04

# Install package dependencies
RUN apt-get update
RUN apt-get install -y \
    libopus0 \
    libopus-dev \
    python3 \
    python3-pip \
    ffmpeg \
    && apt-get clean && rm -rf /var/lib/apt/lists/*
RUN python3 -m pip install --break-system-packages -U "yt-dlp[nightly]"

COPY --from=build /usr/local/cargo/bin/mee6 /usr/local/bin/mee6
CMD ["mee6"]
