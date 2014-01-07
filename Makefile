all: example toml_test


% : %.rs
	rustc --bin -o $@ $^

clean:
	rm -f example toml_test

