FROM rust:latest

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

# docker run "/usr/src/rusty-hooks/target/release/rusty-hooks" -- prod
#docker run --name epic_jackson --entrypoint /bin/bash rusty-hooks
#docker run --name <CONTAINER_NAME> --entrypoint /bin/bash <IMAGE_NAME>
#docker build -t rusty-hooks .
