# EditorConfig Core

[![Crates.io](https://img.shields.io/crates/v/editorconfig-core.svg)](https://crates.io/crates/editorconfig-core)
[![Documentation](https://docs.rs/editorconfig-core/badge.svg)](https://docs.rs/editorconfig-core/)
[![EditorConfig Core Tests](https://github.com/rotmh/editorconfig-core-rust/actions/workflows/tests.yaml/badge.svg)](https://github.com/rotmh/editorconfig-core-rust/actions/workflows/tests.yaml)

An [EditorConfig] Core passing all the [editorconfig-core-test] tests.

See [the documentation].

## Testing

The EditorConfig Cores testing suite uses CTest and tests CLI built around Core
libraries.

This crate contains an auxiliary EditorConfig CLI which was created for testing
purposes, as stated above.

Although it was created for testing, you can use it in your project for
extracting properties of a path from the shell.

Example usage:

```sh
cargo build --bin editorconfig
editorconfig ./README.md
```

## License

Licensed under the MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT).

[EditorConfig]: https://editorconfig.org/
[editorconfig-core-test]: https://github.com/editorconfig/editorconfig-core-test
[the documentation]: https://docs.rs/editorconfig-core/
