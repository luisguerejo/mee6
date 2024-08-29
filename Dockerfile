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


FROM ubuntu:22.04

# Install package dependencies
RUN apt-get update -y && apt-get install -y libopus0 libopus-dev
COPY --from=build /usr/local/cargo/bin/mee6 /usr/local/bin/mee6
CMD ["mee6"]
