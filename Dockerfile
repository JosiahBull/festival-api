FROM josiahbull/festival-api-base:latest

WORKDIR /app

COPY . .

RUN cargo build --release --bin festival-api

RUN mv /app/target/release/festival-api /app/target/ \
    && rm -rdf /app/target/release \
    && rm -rdf /app/target/debug \
    && rm -rdf /app/src \
    && rm -rdf /app/tests \
    && rm -rdf /app/.git \
    && rm -rdf /app/.github \
    && rm -rdf /app/docker-base \
    && rm -rdf /app/benches \
    && rm -rdf /app/cache/*

ENV ROCKET_ADDRESS 0.0.0.0
ENV ROCKET_PORT 3000

EXPOSE 3000

ENTRYPOINT ["/app/target/festival-api"]