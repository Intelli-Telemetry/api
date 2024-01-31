FROM fedora:latest as builder

RUN dnf -y update && \
    dnf -y install openssl-devel ca-certificates pkgconfig protobuf-compiler mold mimalloc && \
    dnf -y groupinstall "Development Tools" && \
    dnf clean all

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /usr/src/intelli-api

COPY . .

RUN cargo update && \
    RUSTFLAGS="-C link-arg=-fuse-ld=mold -C target-cpu=native" cargo build --release

FROM fedora:latest

COPY --from=builder /usr/src/intelli/target/release/intelli-api /usr/local/bin/intelli-api

COPY /migrations /migrations
COPY /certs /certs

CMD ["intelli-api"]
