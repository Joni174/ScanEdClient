FROM rust as planner
WORKDIR /app
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM rust as cacher
WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust as rust_builder
WORKDIR /app
COPY . .
# Copy over the cached dependencies
COPY --from=cacher /app/target target
COPY --from=cacher $CARGO_HOME $CARGO_HOME
RUN cargo build --release

FROM opendronemap/odm as odm
WORKDIR /code
COPY --from=rust_builder /app/target/release/scaned_client .
COPY html html
EXPOSE 8080
ENTRYPOINT ["./scaned_client"]
#ENTRYPOINT ["bash"]