# rust-toml [![Build Status][travis-image]][travis-link]

[travis-image]: https://travis-ci.org/mneumann/rust-toml.png?branch=master
[travis-link]: https://travis-ci.org/mneumann/rust-toml

A [TOML][toml-home] configuration file parser for [Rust][rust-home].

[toml-home]: https://github.com/mojombo/toml
[rust-home]: http://www.rust-lang.org

## Quickstart

Given the following TOML configuration file:

```
# products.toml

[[products]]

id = 1
name = "prod1"

[[products]]

id = 2
name = "prod2'
```

You can access it like in the example below:

```rust
extern mod toml;

fn main() {
    let root = toml::parse_from_file("products.toml");
    let id1  = root.lookup("products.0.id").get_int();
    let name2 = root.lookup("products.1.name").get_str();
    match (id1, name2) {
        (Some(id1), Some(ref name2)) => {
            println!("id1: {}, name2: {}", id1, name2)
        }
        _ => {
            println!("Not found")
        }
    }
}
```

## Conformity

I am using [this test suite][test-suite] to check for conformity to the TOML spec.
You can run it like this (see it's homepage for details on how to install it):

```sh
$HOME/go/local/bin/toml-test rust-toml/toml_test
```

Right now 46 tests pass, while 17 fail. Most of the tests that fail are because
my parser is more loose in what it accepts and what not. For exaple I allow
whitespace and newlines at almost any location, whereas the spec does not.

[test-suite]: https://github.com/BurntSushi/toml-test

## License

rust-redis is under the MIT license, see LICENSE-MIT for details.

Copyright (c) 2014 by Michael Neumann.
