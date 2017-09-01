FROM ekidd/rust-musl-builder as builder

USER rust

COPY . /home/rust/src

# Need to have the rust user own our copied source files
RUN sudo chown -R rust . &&\
    cargo build --release --verbose

FROM alpine

ARG VCS_REF
ARG CA_CERT
ARG BUILD_DATE

LABEL org.label-schema.vcs-ref=$VCS_REF \
      org.label-schema.vcs-url="https://github.com/lloydmeta/rusqbin" \
      org.label-schema.build-date=$BUILD_DATE

COPY $CA_CERT /etc/ssl/certs/
COPY --from=builder /home/rust/src/target/x86_64-unknown-linux-musl/release/rusqbin /rusqbin
COPY entry.sh /entry.sh

RUN addgroup -S rusqbinuser &&\
    adduser -S -g rusqbinuser rusqbinuser &&\
    chown -R rusqbinuser /etc/ssl/certs/ &&\
    chown rusqbinuser /rusqbin &&\
    chown rusqbinuser /entry.sh

USER rusqbinuser

EXPOSE 9999

ENTRYPOINT ["/entry.sh", "/rusqbin"]