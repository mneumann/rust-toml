compile:
	rustpkg build -O toml
	rustpkg build -O testsuite
	rustpkg build examples/simple
	rustpkg build examples/decoder

install: compile
	rustpkg install toml
	rustpkg install testsuite
	rustpkg install examples/simple
	rustpkg install examples/decoder

test: install
	./bin/testsuite ./tests

clean:
	rustpkg clean
	$(RM) -r bin/ build/ lib/
