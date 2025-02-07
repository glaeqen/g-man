FROM alpine:latest

# Dependencies
RUN apk add --no-cache git openssh-client
COPY ./target/x86_64-unknown-linux-musl/release/g-man /usr/bin/g-man

# When starting a container:
# 1. Mount a config file properly (`/etc/g-man/config.toml`)
# 2. Mount the SSH-related files (known_hosts and key pairs)
#    in the `/root/.ssh` so it can establish connection to
#    Gerrit's SSHD.
ENTRYPOINT ["g-man", "/etc/g-man/config.toml"]
