FROM ubuntu:20.04

RUN apt-get update \
    && apt-get install -y libpq-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY native/target/release/cosmicverge-server .env ./
COPY web/static/ ./static/

EXPOSE 7879/tcp

ENV RUST_BACKTRACE=1

CMD ./cosmicverge-server serve