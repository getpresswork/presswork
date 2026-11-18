FROM ubuntu:24.04@sha256:73b999393d07f2bf65a40373778a44dc9a6d54ad7718d29247d8ff9413c4d6a8

LABEL org.opencontainers.image.title="Presswork server" \
      org.opencontainers.image.description="The Presswork collaboration server" \
      org.opencontainers.image.source="https://github.com/getpresswork/presswork" \
      org.opencontainers.image.documentation="https://getpresswork.github.io/presswork/design/server.html" \
      org.opencontainers.image.licenses="Apache-2.0"

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ocrmypdf tesseract-ocr-eng \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 presswork \
    && useradd --system --uid 10001 --gid presswork --home-dir /var/lib/presswork \
       --no-create-home --shell /usr/sbin/nologin presswork \
    && install --directory --owner presswork --group presswork /var/lib/presswork

COPY presswork-server /usr/local/bin/presswork-server
COPY LICENSE NOTICE /usr/share/doc/presswork-server/

ENV PRESSWORK_LISTEN=0.0.0.0:8080 \
    PRESSWORK_DATA=/var/lib/presswork

USER presswork
WORKDIR /var/lib/presswork
VOLUME /var/lib/presswork
EXPOSE 8080
ENTRYPOINT ["presswork-server"]
CMD ["serve"]
