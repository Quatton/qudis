#################
## build stage ##
#################

FROM --platform=linux/amd64 rust:1-slim-bookworm AS builder
WORKDIR /code

# Download crates-io index and fetch dependency code.
# This step avoids needing to spend time on every build downloading the index
# which can take a long time within the docker context. Docker will cache it.
RUN USER=root cargo init
COPY Cargo.toml Cargo.toml
RUN cargo fetch

# Copy the source code and build the application.
COPY src src

# Build the application.
RUN cargo build --release

#################
## run stage ##
#################

FROM --platform=linux/amd64 debian:bookworm-slim AS runner
WORKDIR /app

# install curl
RUN apt-get update && apt-get install -y curl

# Copy the built binary from the builder stage.
COPY --from=builder /code/target/release/qudis /app/qudis

EXPOSE 8080

# run server
CMD [ "./qudis" ]
