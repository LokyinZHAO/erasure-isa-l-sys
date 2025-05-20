# erasure-isa-l-sys

This is a Rust FFI binding for [`isa-l`](https://github.com/intel/isa-l), which provides optimized low-level functions targeting storage applications in C. 
This project allows you to use raw functions in your Rust applications.

You can use a more rusty crate [`erasure-isa-l`](https://crates.io/crates/erasure-isa-l) that provides a more idiomatic interface to the `isa-l` library.

## Requirements

By default, this crate bundles the source code of `isa-l`, and compiles library during the build process, so you don't need to have `libisal` pre-installed on your system.

However, this crate uses the [`bindgen`](https://crates.io/crates/bindgen) and [`autotools`](https://crates.io/crates/autotools) crates, which depend on the following packages:
- libllvm
- libtool
- autoconf
- automake
- nasm

On Ubuntu-like distributions, you can install these dependencies using:
``` shell
apt install autoconf automake libtool libclang-dev nasm
```

If you want to use the system library instead, you can unset the `bundle` feature. This will make the build process look for the `libisal` library on your system.


## Contributing

Feel free to open an issue. If you've got a fix or feature ready, open a PR. Thanks!

## License

MIT