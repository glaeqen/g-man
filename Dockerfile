# Simple "runner" image for easy use with docker-compose

FROM ubuntu:22.04

# Dependencies
RUN apt-get update && apt-get install -y \
    git \
    openssh-client \
    && rm -rf /var/lib/apt/lists/*

# When starting a container:
# 1. Mount a config file properly so the entrypoint makes sense
# 2. Mount the SSH-related files (known_hosts and key pairs)
#    in the `/root/.ssh`
ENTRYPOINT ["g-man", "/etc/g-man/config.toml"]
