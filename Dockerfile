FROM fedora:latest as builder

RUN dnf -y update && \
    dnf -y install openssl-devel ca-certificates pkgconfig protobuf-compiler mold mimalloc && \
    dnf -y groupinstall "Development Tools" && \
    dnf clean all

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /usr/src/intelli

COPY Cargo.toml Cargo.lock ./

COPY /migrations /usr/src/intelli/migrations
COPY /certs /usr/src/intelli/certs

RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY . .
RUN RUSTFLAGS="-C link-arg=-fuse-ld=mold -C target-cpu=native" cargo build --release

FROM fedora:latest

COPY --from=builder /usr/src/intelli/target/release/intelli /usr/local/bin/intelli

COPY /migrations /migrations
COPY /certs /certs

CMD ["intelli"]