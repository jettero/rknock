
GIT_DIR := $(shell git rev-parse --git-dir)
LAST_FILE := $(GIT_DIR)/info/last-make

default: build # last

last:
	@+ make --no-print-directory "$$(cat $(LAST_FILE) 2>/dev/null || echo run)"

run-% verb-%:
	cargo run --bin $* -- --verbose

%-help:
	@ echo $@ > $(LAST_FILE)
	cargo run --bin $* -- --help

listen: no-listen
	@ echo $@ > $(LAST_FILE)
	cargo run --bin door -- --verbose &

stop no-listen:
	@ echo -n "stop listen: "; pkill -ef target/debug/door || :

spam: listen
	@ echo $@ > $(LAST_FILE)
	for i in msg{1..3}; do echo $$i | nc -q1 -u localhost 20022; done
	@+make --no-print-directory no-listen

five-knock: listen
	@ echo $@ > $(LAST_FILE)
	for i in {1..3}; do sleep 0.5; cargo run; done
	@+make --no-print-directory no-listen

ubuild:
	@ echo build > $(LAST_FILE)
	cargo update
	cargo build

run test build:
	@ echo $@ > $(LAST_FILE)
	cargo $@

clean:
	cargo $@
	git clean -dfx

