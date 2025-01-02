# Dav

Simple CardDav server implemented in Rust.

## Usage

Run the application using:
```
cargo run
```

### Health check

You can check the status of the server using:
```
curl -i http://127.0.0.1:3000/health
```

You should receive a `200 OK`.
