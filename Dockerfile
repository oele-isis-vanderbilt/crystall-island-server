FROM rust:slim AS builder

WORKDIR /app

COPY . .

RUN apt-get update && apt-get install libpq-dev pkg-config g++ libx11-dev libxext-dev libgl1-mesa-dev -y && cargo build --bin crystall-island-server --release

FROM ubuntu:latest

ARG APP=/app

EXPOSE 9000

ENV TZ=Etc/UTC \
    APP_USER=crystall-island-user

RUN groupadd $APP_USER && useradd -g $APP_USER $APP_USER

RUN apt-get update && apt-get install tzdata libpq-dev ca-certificates -y && rm -rf /var/cache/apk/* && update-ca-certificates

WORKDIR $APP

COPY --from=builder /app/target/release/crystall-island-server ./api

RUN chown -R $APP_USER:$APP_USER ${APP}

USER $APP_USER

ENTRYPOINT ["./api"]

CMD ["./api"]