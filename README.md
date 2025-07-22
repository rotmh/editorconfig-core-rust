# EditorConfig Core

[![Crates.io](https://img.shields.io/crates/v/editorconfig-core.svg)](https://crates.io/crates/editorconfig-core)
[![Documentation](https://docs.rs/editorconfig-core/badge.svg)](https://docs.rs/editorconfig-core/)
[![EditorConfig Core Tests](https://github.com/rotmh/editorconfig-core-rust/actions/workflows/tests.yaml/badge.svg)](https://github.com/rotmh/editorconfig-core-rust/actions/workflows/tests.yaml)

An [EditorConfig] Core passing all the [`editorconfig-core-test`] tests.

See [the documentation].

## Testing

The EditorConfig core test suite ([`editorconfig-core-test`]) uses CTest and
validates CLI tools built on top of core libraries.

This crate includes a simple CLI (`editorconfig`) to support that testing, but
it can also be used as a standalone tool:

```sh
$ cargo build --bin editorconfig

$ # Example usage and output
$ ./target/debug/editorconfig ./README.md

charset=utf-8
end_of_line=lf
```

## License

Licensed under the MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT).

[EditorConfig]: https://editorconfig.org/
[`editorconfig-core-test`]: https://github.com/editorconfig/editorconfig-core-test
[the documentation]: https://docs.rs/editorconfig-core/
