FROM rustlang/rust:nightly

USER root
ENV USER root

# Install package dependencies.
RUN apt-get update \
    && apt-get install -y \
    apt-utils

# Copy source into container
WORKDIR /usr/src/rusty-hooks
COPY . .

# Build the application binary
RUN cargo build --release

