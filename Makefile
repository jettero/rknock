
RELEASES  := $(patsubst %,release-%, $(PLATFORMS))
VERSION   := $(shell git describe --dirty --tags --match 'v[0-9][.]*' | sed -e s/^v// -e s/-g/-/)
GIT_DIR   := $(shell git rev-parse --git-dir)
HEADS     := $(GIT_DIR)/HEAD $(shell git show-ref --heads --tags | sed -e 's,.* ,$(GIT_DIR)/,')

default: build

version: Cargo.toml

Cargo.toml: input.toml Makefile $(HEADS)
	@echo '## THIS FILE IS GENERATED ##' > $@
	@echo '## THIS FILE IS GENERATED ##' >> $@
	@echo '## THIS FILE IS GENERATED ##' >> $@
	@echo '## THIS FILE IS GENERATED ##' >> $@
	sed -e 's/UNKNOWN/$(VERSION)/' $< >> $@ && grep -H ^version $@

build: test

doc run test build: Cargo.toml
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

update: Cargo.toml
	cargo update
	sed -e s/$(VERSION)/UNKNOWN/ $< | grep -v GENERATED > input.toml

ubuild:
	@+make --no-print-directory update
	@+make --no-print-directory build

clean:
	cargo $@
	git clean -dfx

ls list list-release-targets:
	@ for i in $(RELEASES); do echo $$i; done

release:
	cargo build --release --locked

release: $(RELEASES)
