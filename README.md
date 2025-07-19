# EditorConfig Core

An [EditorConfig] Core passing all the [editorconfig-core-test] tests.

See [the documentation].

## Note on the CLI

This package contains a binary crate as well as the library. This binary
contains an EditorConfig CLI which was created for testing purposes, as
[editorconfig-core-test] operates on CLIs.

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
