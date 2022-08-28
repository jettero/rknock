
PLATFORMS := x86_64-unknown-linux-gnu x86_64-apple-darwin aarch64-apple-darwin
RELEASES  := $(patsubst %,release-%, $(PLATFORMS))
VERSION   := $(shell git describe --dirty --tags --long --match v[0-9][.]*)
GIT_DIR   := $(shell git rev-parse --git-dir)
HEADS     := $(GIT_DIR)/HEAD $(GIT_DIR)/$(shell git show-ref --heads | cut -d' ' -f2)

default: build

Cargo.toml: input.toml Makefile $(HEADS)
	@echo '## THIS FILE IS GENERATED ##' > $@
	@echo '## THIS FILE IS GENERATED ##' >> $@
	@echo '## THIS FILE IS GENERATED ##' >> $@
	@echo '## THIS FILE IS GENERATED ##' >> $@
	sed -e 's/UNKONWN/$(VERSION)/' $< >> $@

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

update ubuild:
	cp input.toml Cargo.toml
	cargo update
	cp Cargo.toml input.toml
	git add Cargo.lock input.toml
	git commit -m "cargo update" Cargo.lock input.toml
	@+make --no-print-directory build

clean:
	cargo $@
	git clean -dfx

ls list list-release-targets:
	@ for i in $(RELEASES); do echo $$i; done

release-%:
	cargo build --release --locked --target $*

release: $(RELEASES)
