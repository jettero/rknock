
default: build

run-% verb-%:
	cargo run --bin $* -- --verbose

%-help:
	cargo run --bin $* -- --help

listen: no-listen
	cargo run --bin door -- --verbose &

stop no-listen:
	fuser -vkn udp 20022 || :
	@echo; echo; echo

spam: listen
	for i in msg{1..3}; do echo $$i | nc -q1 -u localhost 20022; done
	@+make --no-print-directory no-listen

five-knock: listen
	for i in {1..3}; do sleep 0.5; cargo run; done
	@+make --no-print-directory no-listen

update ubuild:
	cargo update
	@+make --no-print-directory build
	git add Cargo.lock Cargo.toml
	git commit -m "cargo update" Cargo.lock Cargo.toml

run test build:
	cargo $@

release release-build:
	cargo build --release --locked

clean:
	cargo $@
	git clean -dfx
