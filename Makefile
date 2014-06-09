.PHONY: lib all examples test clean

RUSTC?=rustc

LIBNAME := $(shell ${RUSTC} --crate-file-name src/toml/lib.rs)

all: lib examples test

lib: lib/$(LIBNAME)

lib/$(LIBNAME): src/toml/lib.rs
	@mkdir -p lib
	${RUSTC} -O --out-dir lib $<

test: bin/testsuite
	./bin/testsuite ./tests

bin/testsuite: src/testsuite/main.rs lib/$(LIBNAME)
	@mkdir -p bin
	${RUSTC} -O -o bin/testsuite -L lib $<

examples: bin/simple bin/decoder

bin/simple: src/examples/simple/main.rs lib/$(LIBNAME)
	@mkdir -p bin
	${RUSTC} -o bin/simple -L lib $<

bin/decoder: src/examples/decoder/main.rs lib/$(LIBNAME)
	@mkdir -p bin
	${RUSTC} -o bin/decoder -L lib $<

clean:
	-$(RM) -r bin
	-$(RM) -r lib
