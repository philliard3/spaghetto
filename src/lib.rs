//! # spaghetto
//! **spaghetto** is a library for creating double-ended data structures that can be rolled out or consumed from either side.
//! This includes the base structure [`DeVec`], a double-ended [`Vec`] that can be used as a deque, and [`DeString`], a double-ended alternative to [`String`].
//!
//! <br>
//!
//! ## DeVec
//! A [`DeVec`] can be used as a double-ended queue, but with the added benefit of being able to get a single contiguous slice of the entire structure.
//! ```
//! use spaghetto::DeVec;
//! let mut devec : DeVec<i32> = DeVec::with_capacity(5);
//! devec.push_back(1);
//! devec.push_front(2);
//! devec.push_back(3);
//! assert_eq!(devec.pop_front(), Some(2));
//! devec.pop_front();
//! // Now let's overload one side.
//! // This forces a DeVec to reallocate in order to keep the elements contiguous.
//! // With a VecDeque this would no longer be a contiguous allocation
//! // because we moved the front of the queue into the middle of the buffer
//! // and pushed it out past the end of the buffer.
//! // It would need to wrap around.
//! for i in 0..3 {
//!    devec.push_back(i);
//! }
//!
//! // we can get a single contiguous slice of the entire DeVec without having to shift elements.
//! let no_mutation = &devec;
//! let slice = no_mutation.as_slice();
//! ```
//!
//! ## DeString
//! A [`DeString`] can be used as a double-ended string, and because of this, we can efficiently remove extra whitespace from either side, mutating in place and maintining a single contiguous string slice without the cost of shifting like with a String.
//!
//! ```
//! use spaghetto::DeString;
//!
//! let mut destring = DeString::from("  hello world  ");
//! destring.mut_trim_front();
//!
//! // It is contiguous and also mutated its starting position!
//! assert_eq!(destring.as_str(), "hello world  ");
//! ```

////////////////////////////////////////////////////////////////////////////////

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod destring;
pub use destring::DeString;

pub mod settings;
pub use settings::*;

pub mod devec;
pub use devec::DeVec;

#[cfg(test)]
mod property_tests;

// a silly little guy
/// The default config for a [`DeVec<T>`], named after a single piece of spaghetti.
pub type Spaghetto<T> = DeVec<T, FrontToBack, Middle>;
