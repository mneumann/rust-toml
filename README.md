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
[global]

# ...

[db]

  [[db.products]]

  id = 1
  name = "prod1"

  [[db.products]]

  id = 2
  name = "prod2"
```

You can access it like in the example below:

```rust
extern mod toml = "toml#0.1";

fn main() {
    let root = toml::parse_from_file("products.toml").unwrap();
    let id1  = root.lookup("db.products.0.id").get_int();
    let name2 = root.lookup("db.products.1.name").get_str();
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
## Benchmark

I did a pretty non-scientific benchmark against [go-toml] for a 
very large document (3 million lines). Not that it would matter
in any way, but it shows that [rust-toml] is about three times
as fast.

[go-toml]: https://github.com/pelletier/go-toml
[rust-toml]: https://github.com/mneumann/rust-toml

## Conformity

I am using [this test suite][test-suite] to check for conformity to the TOML spec.
The test cases are also included in this git repo, you can run them with
this command:

```sh
./bin/testsuite ./tests
```

Alternatively you can run it with the test runner from the original
[test-suite][test-suite] using this command (see it's [homepage][test-suite]
for details on how to install it):

```sh
$HOME/go/local/bin/toml-test rust-toml/bin/testsuite
```

Right now all 63 tests pass, none fails. 

[test-suite]: https://github.com/BurntSushi/toml-test

## License

rust-toml is under the MIT license, see [LICENSE-MIT][license] for details.

[license]: LICENSE-MIT

Copyright (c) 2014 by Michael Neumann.
