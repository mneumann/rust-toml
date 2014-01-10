compile:
	rustpkg build -O toml
	rustpkg build -O testsuite
	rustpkg build examples/simple

install: compile
	rustpkg install toml
	rustpkg install testsuite
	rustpkg install examples/simple

test: install
	./bin/testsuite ./tests

clean:
	rustpkg clean
