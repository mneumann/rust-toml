.PHONY: lib all examples test clean

LIBNAME := $(shell rustc --crate-file-name src/toml/lib.rs)

all: lib examples test

lib: lib/$(LIBNAME)

lib/$(LIBNAME): src/toml/lib.rs
	@mkdir -p lib
	rustc -O --out-dir lib $<

test: bin/testsuite
	./bin/testsuite ./tests

bin/testsuite: src/testsuite/main.rs lib
	@mkdir -p bin
	rustc -O -o bin/testsuite -L lib $<

examples: bin/simple bin/decoder

bin/simple: src/examples/simple/main.rs lib
	@mkdir -p bin
	rustc -o bin/simple -L lib $<

bin/decoder: src/examples/decoder/main.rs lib
	@mkdir -p bin
	rustc -o bin/decoder -L lib $<

clean:
	-$(RM) -r bin
	-$(RM) -r lib
