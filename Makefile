
PLATFORMS := x86_64-unknown-linux-gnu x86_64-apple-darwin aarch64-apple-darwin
RELEASES := $(patsubst %,release-%, $(PLATFORMS))

default: build

Cargo.toml: input.toml

build: test

doc run test build:
	cargo $@

%-help:
	cargo run --bin $* -- --help

knock:
	cargo run --bin knock -- --verbose

door listen: no-listen
	cargo run --bin door -- --verbose

listen-bg: no-listen
	cargo run --bin door -- --verbose & echo

stop no-listen:
	fuser -vkn udp 20022 || :
	@echo

spam: listen-bg
	for i in msg{1..3}; do sleep 0.5; echo $$i | nc -q1 -u localhost 20022; done
	@+make --no-print-directory no-listen

five-knock: listen-bg
	for i in {1..3}; do sleep 0.5; cargo run --bin knock; done
	@+make --no-print-directory no-listen

update ubuild:
	cargo update
	@+make --no-print-directory build
	git add Cargo.lock Cargo.toml
	git commit -m "cargo update" Cargo.lock Cargo.toml

clean:
	cargo $@
	git clean -dfx

ls list list-release-targets:
	@ for i in $(RELEASES); do echo $$i; done

release-%:
	cargo build --release --locked --target $*

release: $(RELEASES)
