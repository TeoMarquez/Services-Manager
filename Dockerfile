FROM rust:1.88-bookworm

RUN apt-get update \
    && apt-get install --yes --no-install-recommends build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY migrations/ migrations/
COPY mock/ mock/

RUN cargo build --locked

CMD ["cargo", "test", "--locked"]
