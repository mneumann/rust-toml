.PHONY: lib all examples test clean

LIBNAME := $(shell rustc --crate-file-name src/toml/lib.rs)

all: lib examples test
	@## Build library and examples, run tests

doc:
	@## Build documentation
	rustdoc src/toml/lib.rs

help:
	@## Show this help
	@grep -A1 ^[a-z].*\: Makefile | sed -r 's/: (.*)$$/:/g' | sed ':a;N;$$!ba;s/:\n//g' | sed s,\\#,\\t,g | sed s,@,,g | grep -v \\--

lib: lib/$(LIBNAME)
	@## Build library

lib/$(LIBNAME): src/toml/lib.rs
	@# --
	@mkdir -p lib
	rustc -O --out-dir lib $<

test: bin/testsuite
	@## Run tests
	./bin/testsuite ./tests

bin/testsuite: src/testsuite/main.rs lib/$(LIBNAME)
	@# --
	@mkdir -p bin
	rustc -O -o bin/testsuite -L lib $<

examples: bin/simple bin/decoder
	@# Build examples

bin/simple: src/examples/simple/main.rs lib/$(LIBNAME)
	@# --
	@mkdir -p bin
	rustc -o bin/simple -L lib $<

bin/decoder: src/examples/decoder/main.rs lib/$(LIBNAME)
	@# --
	@mkdir -p bin
	rustc -o bin/decoder -L lib $<

clean:
	@## Remove compiled sources
	-$(RM) -r bin
	-$(RM) -r lib
	-$(RM) -r doc

version:
	@## Display version of source code
	git describe
