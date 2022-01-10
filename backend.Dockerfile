FROM rust:latest

WORKDIR /app

COPY . .

RUN apt-get update \
    && apt-get install -y festival ffmpeg flite cmake

RUN cargo build --release

ENV ROCKET_HOST 0.0.0.0
ENV ROCKET_PORT 3000

EXPOSE 3000

ENTRYPOINT ["/app/target/release/festival-api"]