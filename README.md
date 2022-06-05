# Unarray

Utilities for working with uninitialized arrays

This crate provides a few sets of APIs:

## `uninit_buf` and `mark_initialized`

These are a pair of functions which are generally used as follows:
 - stack-allocate an uninitialized array with `uninit_buf`
 - initialize each element
 - unsafely convert it to an initialized array with `mark_initialized`

For example:
```rust
use unarray::*;

fn main() {
  let mut buffer = uninit_buf::<i32; 10>();

  for slot in &mut buffer {
    slot.write(123);
  }

  let array = unsafe { mark_initialized(buffer) };

  assert_eq!(array, [123; 10]);
}
```

This is simple to understand, but still requires `unsafe`, which is hard to justify in many cases

## `build_array_*`

[![Docs badge]][docs.rs]



## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.



[Docs badge]: https://img.shields.io/badge/docs.rs-rustdoc-green
[docs.rs]: https://docs.rs/unarray/
