FROM josiahbull/festival-api-base:latest

WORKDIR /app

COPY . .

RUN cargo build --release --bin festival-api

RUN mv /app/target/release/festival-api /app/target/ \
    && rm -rd /app/target/release \
    && rm -rd /app/src \
    && rm -rd /app/tests \
    && rm -rd /app/.git \
    && rm -rd /app/.github \
    && rm -rd /app/docker-base \
    && rm -rd /app/benches \
    && rm -rd /app/cache/*

ENV ROCKET_ADDRESS 0.0.0.0
ENV ROCKET_PORT 3000

EXPOSE 3000

ENTRYPOINT ["/app/target/festival-api"]