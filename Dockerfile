FROM rustlang/rust:nightly as builder

WORKDIR /App
COPY . /App/

RUN cargo build --release

FROM alpine
WORKDIR /App
COPY --from=builder /App/target/release/backend_vr_shahe /App/backend_vr_shahe
CMD ["./backend_vr_shahe"]
