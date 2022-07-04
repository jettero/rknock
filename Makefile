
default: last

last:
	@+ make --no-print-directory "$$(cat .last 2>/dev/null || echo run)"

door:
	cargo run --bin door -- --verbose

door-help:
	@ echo $@ > .last
	cargo run --bin door -- --help

listen: no-listen
	@ echo $@ > .last
	cargo run --bin door -- --verbose &

no-listen:
	@ echo -n "stop listen: "; killall -v target/debug/door || true

spam: listen
	@ echo $@ > .last
	for i in msg1 msg:2 msg3; do echo $$i | nc -q1 -u localhost 20022; done
	@+make --no-print-directory no-listen

run: listen
	@ echo $@ > .last
	for i in 1 2 3; do sleep 0.5; cargo run --bin knock; done
	@+make --no-print-directory no-listen

ubuild:
	@ echo $@ > .last
	cargo update
	cargo build

build:
	@ echo $@ > .last
	cargo $@

clean:
	cargo clean
	git clean -dfx

