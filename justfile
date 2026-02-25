set shell := ["powershell", "-Command"]

default: run

run:
    cargo run

build:
    cargo build

release:
    cargo build --release

check:
    cargo build
    cargo clippy
    cargo fmt --check

fmt:
    cargo fmt

kill:
    taskkill //F //IM provision.exe || true
