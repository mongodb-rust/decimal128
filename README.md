<h1 align="center">decimal 128</h1>
<div align="center">
 <strong>
   128-bit wide floating point implementation for Rust
 </strong>
</div>

<br />

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/decimal128">
    <img src="https://img.shields.io/crates/v/decimal128.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/decimal128">
    <img src="https://img.shields.io/crates/d/decimal128.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs.rs docs -->
  <a href="https://docs.rs/decimal128">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
</div>

## Installation
```shell
cargo add decimal128
```

## Usage
This crate is a work-in-progress and does not have all applicable methods implemented as per [IEEE Standard for Floating-Point Arithmetic](https://ieeexplore.ieee.org/document/4610935) and [MongoDB Decimal128 BSON type](https://github.com/mongodb/specifications/blob/master/source/bson-decimal128/decimal128.rst). The following methods are currently implemented:
- `Decimal128.from_raw_bytes`
- `Decimal128.zero`
- `Decimal128.is_nan`
- `Decimal128.is_negative`
- `Decimal128.is_zero`
- `Decimal128.to_string`

```rust
use decimal128;

let vec: [u8; 16] = [
    0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00,
];
let dec128 = Decimal128::from_raw_buf(vec);
let string = dec128.to_string();
assert_eq!("-Infinity".to_string(), string);
```

## License

Licensed under the Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
at your option.
