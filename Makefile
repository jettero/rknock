
GIT_DIR := $(shell git rev-parse --git-dir)
LAST_FILE := $(GIT_DIR)/info/last-make

default: last

last:
	@+ make --no-print-directory "$$(cat $(LAST_FILE) 2>/dev/null || echo run)"

door knock:
	cargo run --bin $@ -- --verbose

%-help:
	@ echo $@ > $(LAST_FILE)
	cargo run --bin $* -- --help

listen: no-listen
	@ echo $@ > $(LAST_FILE)
	cargo run --bin door -- --verbose &

no-listen:
	@ echo -n "stop listen: "; pkill -ef target/debug/door || true

spam: listen
	@ echo $@ > $(LAST_FILE)
	for i in msg1 msg:2 msg3; do echo $$i | nc -q1 -u localhost 20022; done
	@+make --no-print-directory no-listen

run: listen
	@ echo $@ > $(LAST_FILE)
	for i in 1 2 3; do sleep 0.5; cargo run --bin knock; done
	@+make --no-print-directory no-listen

ubuild:
	@ echo $@ > $(LAST_FILE)
	cargo update
	cargo build

build:
	@ echo $@ > $(LAST_FILE)
	cargo $@

clean:
	cargo clean
	git clean -dfx

