//! # Unarray
//!
//! Helper utilities for working with arrays of uninitialized memory.
//!
//! Creating arrays in Rust can be somewhat painful. Currently, your best option in the general
//! case is to allocate your elements in a `Vec`, then convert to an array:
//! ```
//! const LEN: usize = 1000;
//! let mut elements = Vec::with_capacity(LEN);  // heap allocation here
//!
//! for i in 0..LEN {
//!   elements.push(123);
//! }
//!
//! let result: [i32; LEN] = elements.try_into().unwrap();
//! ```
//! This needlessly allocates space on the heap, which is then immediately freed. If your type
//! implements `Copy`, and has a sensible default value, you can avoid this allocation by creating
//! an array literal (e.g. `[0; 1000]`), then iterating over each element and setting it, but this
//! also incurrs an unnecessary initialization cost. Why set each element to `0`, then set it
//! again, when you could just set it once?
//!
//! The lowest-level tools provided by this library are the pair of functions: [`uninit_buf`] and
//! [`mark_initialized`]. These are ergonomic wrappers around the [`std::mem::MaybeUninit`] type.
//! Roughly speaking, most uses of these functions will follow the following steps:
//!  - Stack-allocate a region of uninitialized memory with [`uninit_buf`]
//!  - Initialize each element
//!  - Unsafely declare that all elements are initialized using [`mark_initialized`]
//!
//! For example:
//! ```
//! # use unarray::*;
//! let mut buffer = uninit_buf();  
//!
//! for elem in &mut buffer {
//!   elem.write(123);  
//! }
//!
//! let result = unsafe { mark_initialized(buffer) };
//! assert_eq!(result, [123; 1000]);
//! ```
//! These functions closely map onto tools provided by [`std::mem::MaybeUninit`], so should feel
//! familiar. However, [`mark_initialized`] is an unsafe function, since it's possible to create
//! uninitialized values that aren't wrapped in `MaybeUninit`. It's up to the programmer to make
//! sure every element has been initialized before calling [`mark_initialized`], otherwise it's UB.
//!
//! For this, there are also fully safe APIs that cover some of the common patterns via an
//! extension trait on `[T; N]`:
//! ```
//! # use unarray::*;
//! // mapping an array
//! let numbers = [1, 2, 3];
//! let doubled = numbers.map(|i| i * 2);
//! assert_eq!(doubled, [2, 4, 6]);
//!
//! // mapping an array via a `Result`
//! let strings = ["123", "234"];
//! let numbers = strings.map_result(|s| s.parse());
//! assert_eq!(numbers, Ok([123, 234]));
//!
//! let bad_strings = ["123", "uh oh"];
//! let result = bad_strings.map_result(|s| s.parse::<i32>());
//! assert!(result.is_err());  // since one of the element fails, the whole operation fails
//! ```
//! There is also `map_option` for functions which return an `Option`
//!
//! Finally, it's often the case that you want to initialize each array element based on its index.
//! For that, [`build_array`] takes a const generic length, and a function that takes an index and
//! returns an element, and builds the array for you:
//! ```
//! use unarray::*;
//! let array: [usize; 5] = build_array(|i| i * 2);
//! assert_eq!(array, [0, 2, 4, 6, 8]);
//! ```
//! There are also variants that allow fallibly constructing an array, via [`build_array_result`]
//! or [`build_array_option`], similar to [`UnarrayArrayExt::map_result`] and [`UnarrayArrayExt::map_option`]

#![deny(clippy::missing_safety_doc, missing_docs)]
use std::mem::MaybeUninit;

mod build;
mod map;
#[cfg(test)]
mod tests;

pub use build::{build_array, build_array_option, build_array_result};
pub use map::UnarrayArrayExt;

/// Convert a `[MaybeUninit<T>; N]` to a `[T; N]`
///
/// ```
/// # use unarray::*;
/// let mut buffer = uninit_buf::<i32, 1000>();
///
/// for elem in &mut buffer {
///   elem.write(123);
/// }
///
/// let result = unsafe { mark_initialized(buffer) };
/// assert_eq!(result, [123; 1000])
/// ```
///
/// This largely acts as a workaround to the fact that [`std::mem::transmute`] cannot be used with
/// const generic arrays, as it can't prove they have the same size (even when intuitively they are
/// the same, e.g. `[i32; N]` and `[u32; N]`).
///
/// # Safety
///
/// Internally, this uses [`std::mem::transmute_copy`] to convert a `[MaybeUninit<T>; N]` to `[T; N]`.
/// As such, you must make sure every element has been initialized before calling this function. If
/// there are uninitialized elements in `src`, these will be converted to `T`s, which is UB. For
/// example:
/// ```no_run
/// # use unarray::*;
/// // ⚠️ This example produces UB ⚠️
/// let bools = uninit_buf::<bool, 10>();
/// let uh_oh = unsafe { mark_initialized(bools) };  // UB: creating an invalid instance
/// if uh_oh[0] {                                    // double UB: reading from unintiailized memory
///   // ...
/// }
/// ```
/// Even if you never use a value, it's still UB. This is especially true for types with
/// [`std::ops::Drop`] implementations:
/// ```no_run
/// # use unarray::*;
/// // ⚠️ This example produces UB ⚠️
/// let strings = uninit_buf::<String, 10>();
/// let uh_oh = unsafe { mark_initialized(strings) };  // UB: creating an invalid instance
///
/// // uh_oh is dropped here, freeing memory at random addresses
/// ```
pub unsafe fn mark_initialized<T, const N: usize>(src: [MaybeUninit<T>; N]) -> [T; N] {
    std::mem::transmute_copy::<[MaybeUninit<T>; N], [T; N]>(&src)
}

/// Create an array of unintialized memory
///
/// This function is just a safe wrapper around `MaybeUninit::uninit().assume_init()`, which is
/// safe when used to create a `[MaybeUninit<T>; N]`, since this type explicitly requires no
/// initialization
///
/// ```
/// # use unarray::*;
/// let mut buffer = uninit_buf::<i32, 1000>();
///
/// for elem in &mut buffer {
///   elem.write(123);
/// }
///
/// let result = unsafe { mark_initialized(buffer) };
/// assert_eq!(result, [123; 1000])
/// ```
pub fn uninit_buf<T, const N: usize>() -> [MaybeUninit<T>; N] {
    // SAFETY:
    // This is safe because we are assuming that a `[MaybeUninit<T>; N]` is initialized. However,
    // since `MaybeUninit` doesn't require initialization, doing nothing counts as "initializing",
    // so this is always safe
    unsafe { MaybeUninit::uninit().assume_init() }
}
