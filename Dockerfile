FROM ubuntu:24.04

# Dependencies
RUN apt-get update && apt-get install -y \
    git \
    openssh-client \
    && rm -rf /var/lib/apt/lists/*

COPY ./target/x86_64-unknown-linux-musl/release/g-man /usr/bin/g-man

# When starting a container:
# 1. Mount a config file properly (`/etc/g-man/config.toml`)
# 2. Mount the SSH-related files (known_hosts and key pairs)
#    in the `/root/.ssh` so it can establish connection to
#    Gerrit's SSHD.
ENTRYPOINT ["g-man", "/etc/g-man/config.toml"]
