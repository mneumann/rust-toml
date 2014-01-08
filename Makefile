build:
	rustpkg build -O toml
	rustpkg build -O testsuite
	rustpkg build examples/simple

install: build
	rustpkg install toml
	rustpkg install testsuite
	rustpkg install examples/simple

clean:
	rustpkg clean
