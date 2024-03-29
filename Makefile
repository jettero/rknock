VERSION   := $(shell git describe --dirty --tags --match 'v[0-9][.]*' | sed -e s/^v// -e s/-g/-/)
GIT_DIR   := $(shell git rev-parse --git-dir)
HEADS     := $(GIT_DIR)/HEAD $(shell git show-ref --heads --tags | sed -e 's,.* ,$(GIT_DIR)/,')
PREFIX    := /usr/local

default: test

version: Cargo.toml

Cargo.toml: input.toml Makefile $(HEADS)
	@ echo making $@ using $< as input
	@ (flock -x 9; chmod -c 0600 $@; \
       (echo '# THIS FILE IS GENERATED #'; \
        sed -e 's/^#.*//' -e 's/UNKNOWN/$(VERSION)/' $<; \
        echo '# THIS FILE IS GENERATED #') \
			| grep . > $@ \
				&& grep -H ^version $@; \
	  chmod -c 0444 $@) 9>/tmp/cargo.lockfile

doc run test build: Cargo.toml
	cargo $@ --color=always

clippy lint: Cargo.toml
	cargo clippy

auto-lint: Cargo.toml
	cargo clippy --allow-dirty --fix

%-help: Cargo.toml
	cargo run --bin $* -- --help

knock: Cargo.toml
	cargo run --bin knock -- --verbose

door listen: no-listen Cargo.toml
	cargo run --bin door -- --verbose

listen-bg: no-listen Cargo.toml
	cargo run --bin door -- --verbose & echo

stop no-listen:
	fuser -vkn udp 20022 || :
	@echo

spam: listen-bg Cargo.toml
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

release:
	cargo build --release

target/debug/%: src/%.rs src/lib.rs Cargo.toml
	cargo build --bin $*

target/release/%: src/%.rs src/lib.rs Cargo.toml
	cargo build --bin $* --release

%/bin/rknock: target/release/knock
	sudo install -o 0 -g 0 -m 0755 -v $< $@

%/bin/rk_door: target/release/door
	sudo install -o 0 -g 0 -m 0755 -v $< $@

install: $(PREFIX)/bin/rknock $(PREFIX)/bin/rk_door

watch-run: last-action
	gh run watch $$(< .last-action)

workflow-run-% run-workflow-% actions-% action-%: .github/workflows/%.yaml
	gh workflow run $*
	@echo wait around for 5 seconds
	@+make --no-print-directory watch-run
