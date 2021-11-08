FROM rust:latest

WORKDIR /app

COPY . .

RUN apt-get update \
    && apt-get install -y festival sox libsox-fmt-mp3 postgresql cmake \
    && wget http://www.festvox.org/packed/festival/2.5/voices/festvox_cmu_us_aew_cg.tar.gz \
    && mkdir -p /usr/share/festival/voices/us/ \
    && tar -xf festvox_cmu_us_aew_cg.tar.gz \
    && cp -r festival/lib/voices/us/cmu_us_aew_cg/ /usr/share/festival/voices/us/ \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install diesel_cli --no-default-features --features postgres

#TODO: Use cargo-chef to improve the build speed of this
#TODO: create an image for diesel_cli, so that we don't have to manually rebuild it each time.
RUN cargo build --release

ENV ROCKET_HOST 0.0.0.0
ENV ROCKET_PORT 3000

EXPOSE 3000

ENTRYPOINT ["/app/initalize_server.sh"]