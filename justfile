build:
    cargo build

run *ARGS:
    cargo run -- {{ARGS}}

release:
    cargo build --release
