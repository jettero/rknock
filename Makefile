
default: last

last:
	@+ make --no-print-directory "$$(cat .last 2>/dev/null || echo run)"

door:
	cargo run --bin door -- --verbose

door-help:
	@ echo $@ > .last
	cargo run --bin door -- --help

run:
	@ echo $@ > .last
	cargo run --bin door -- --verbose & (sleep 0.5; cargo run --bin knock)
	@ sleep 0.5; echo -n make sure no doors are left running:; killall -v target/debug/door || true

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

