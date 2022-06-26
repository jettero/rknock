

default: run

run:
	cargo run --bin door & (sleep 0.5; cargo run --bin knock)

build:
	cargo $@
