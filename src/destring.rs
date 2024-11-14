//! A module to hold [`DeString`], a string type that uses a [`DeVec`] internally.
//!

use super::DeVec;

/// A string type that uses a [`DeVec`] internally.
/// This type is similar to [`String`] but uses a [`DeVec<u8>`] internally instead of a [`Vec<u8>`].
/// This allows it to use the `DeVec`'s double-endedness to provide efficient push/pop operations on both ends, while managing a single contiguous [string slice](str).
///
/// # Examples
/// ```
/// # use spaghetto::DeString;
/// let mut destring = DeString::new();
/// destring.push_str_back("hello");
/// destring.push_str_front("world ");
/// assert_eq!(destring.as_str(), "world hello");
/// ```
pub struct DeString {
    devec: DeVec<u8, crate::FrontToBack, crate::Middle>,
}

impl DeString {
    /// Creates a new empty `DeString`.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let destring = DeString::new();
    /// assert_eq!(destring.as_str(), "");
    /// ```
    #[inline]
    pub const fn new() -> Self {
        DeString {
            devec: DeVec::new(),
        }
    }

    /// Creates a new `DeString` with at least the specified capacity.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let destring = DeString::with_capacity(10);
    /// assert!(destring.capacity() >= 10);
    /// ```
    #[inline]
    pub fn with_capacity(cap: usize) -> Self {
        DeString {
            devec: DeVec::with_capacity(cap),
        }
    }

    /// Appends a character to the end of the `DeString`.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let mut destring = DeString::from("he");
    /// destring.push_char_back('l');
    /// destring.push_char_back('l');
    /// destring.push_char_back('o');
    /// assert_eq!(destring.as_str(), "hello");
    /// ```
    #[inline]
    pub fn push_char_back(&mut self, c: char) {
        let mut buf = [0; 4];
        self.devec.reserve_back(4);
        let bytes_written = c.encode_utf8(&mut buf).as_bytes();
        for &b in bytes_written {
            self.devec.push_back(b);
        }
    }

    /// Appends a character to the front of the `DeString`.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let mut destring = DeString::from("ello");
    /// destring.push_char_front('h');
    /// assert_eq!(destring.as_str(), "hello");
    /// ```
    #[inline]
    pub fn push_char_front(&mut self, c: char) {
        let mut buf = [0; 4];
        self.devec.reserve_front(4);
        let bytes_written = c.encode_utf8(&mut buf).as_bytes();
        for &b in bytes_written.iter().rev() {
            self.devec.push_front(b);
        }
    }

    /// Removes the last character from the `DeString` and returns it, if it exists.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let mut destring = DeString::from("hello");
    /// assert_eq!(destring.pop_back(), Some('o'));
    /// assert_eq!(destring.pop_back(), Some('l'));
    /// assert_eq!(destring.pop_back(), Some('l'));
    /// assert_eq!(destring.as_str(), "he");
    /// assert_eq!(destring.pop_back(), Some('e'));
    /// assert_eq!(destring.pop_back(), Some('h'));
    /// assert_eq!(destring.pop_back(), None);
    /// assert_eq!(destring.as_str(), "");
    /// ```
    #[inline]
    pub fn pop_back(&mut self) -> Option<char> {
        let my_str = self.as_str();
        let len = self.devec.len();
        let mut char_indices = my_str.char_indices();
        let (last_index, last_char) = char_indices.next_back()?;
        let drain = self.devec.drain(last_index..len);
        drop(drain);
        Some(last_char)
    }

    /// Removes the first character from the `DeString` and returns it, if it exists.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let mut destring = DeString::from("hello");
    /// assert_eq!(destring.pop_front(), Some('h'));
    /// assert_eq!(destring.pop_front(), Some('e'));
    /// assert_eq!(destring.pop_front(), Some('l'));
    /// assert_eq!(destring.as_str(), "lo");
    /// assert_eq!(destring.pop_front(), Some('l'));
    /// assert_eq!(destring.pop_front(), Some('o'));
    /// assert_eq!(destring.pop_front(), None);
    /// assert_eq!(destring.as_str(), "");
    /// ```
    #[inline]
    pub fn pop_front(&mut self) -> Option<char> {
        let my_str = self.as_str();
        let mut char_indices = my_str.char_indices();
        let (_, first_char) = char_indices.next()?;
        let second_index = first_char.len_utf8();
        let drain = self.devec.drain(0..second_index);
        drop(drain);
        Some(first_char)
    }

    /// Appends the contents of a string to the end of the `DeString`.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let mut destring = DeString::from("hello");
    /// destring.push_str_back(" world");
    /// assert_eq!(destring.as_str(), "hello world");
    /// ```
    #[inline]
    pub fn push_str_back(&mut self, s: &str) {
        // for c in s.chars() {
        //     self.push_char(c);
        // }

        // we can do better
        self.devec.reserve_back(s.len());
        for &b in s.as_bytes() {
            self.devec.push_back(b);
        }
        // TODO: we may be able to do even better if we just copy the bytes directly
        // TODO: (this means use the DeVec's buffer directly and copy from slice)
    }

    /// Prepends the contents of a string to the front of the DeString.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let mut destring = DeString::from("world");
    /// destring.push_str_front("hello ");
    /// assert_eq!(destring.as_str(), "hello world");
    /// ```
    #[inline]
    pub fn push_str_front(&mut self, s: &str) {
        // for c in s.chars().rev() {
        //     self.push_char(c);
        // }

        // we can do better
        self.devec.reserve_front(s.len());
        for &b in s.as_bytes().iter().rev() {
            self.devec.push_front(b);
        }
        // TODO: we can do even better if we just copy the bytes directly
    }

    /// Returns the number of bytes in the `DeString`.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let destring = DeString::from("hello");
    /// assert_eq!(destring.len(), 5);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.devec.len()
    }

    /// Returns the number of bytes that can be stored in the DeString without reallocating (assuming perfect pushing to both sides).
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let destring = DeString::with_capacity(10);
    /// assert!(destring.capacity() >= 10);
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        self.devec.capacity()
    }

    /// Returns true if the `DeString` is empty.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let destring = DeString::new();
    /// assert!(destring.is_empty());
    /// ```
    /// ```
    /// # use spaghetto::DeString;
    /// let destring = DeString::from("hello");
    /// assert!(!destring.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.devec.is_empty()
    }

    /// Removes all bytes from the `DeString`.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let mut destring = DeString::from("hello");
    /// destring.clear();
    /// assert!(destring.is_empty());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.devec.clear();
    }

    /// Returns a reference to the string slice that the `DeString` manages.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let destring = DeString::from("hello");
    /// assert_eq!(destring.as_str(), "hello");
    /// ```
    #[inline]
    pub fn as_str(&self) -> &str {
        // SAFETY: we know that the `DeString`'s buffer is a valid UTF-8 string because we only ever push valid UTF-8 bytes to it in the form of `char`s and `&str`s
        unsafe { std::str::from_utf8_unchecked(&self.devec) }
    }

    /// Returns a mutable reference to the string slice that the `DeString` manages.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let mut destring = DeString::from("hellO");
    /// let destr = destring.as_mut_str();
    /// destr.make_ascii_lowercase();
    /// assert_eq!(destring.as_str(), "hello");
    /// ```
    #[inline]
    pub fn as_mut_str(&mut self) -> &mut str {
        unsafe { std::str::from_utf8_unchecked_mut(&mut self.devec) }
    }

    /// Returns a reference to the underlying [`DeVec<u8>`] buffer.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let destring = DeString::from("hello");
    /// assert_eq!(destring.as_bytes(), b"hello");
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.devec
    }

    /// Returns a mutable reference to the underlying [`DeVec<u8>`] buffer.
    /// This is useful for when you want to manipulate the buffer directly.
    /// # Safety
    /// This function is unsafe because it exposes the internal buffer of the `DeString`.
    /// It is possible to corrupt the `DeString` by manipulating the buffer directly.
    /// It is also possible to manipulate it to create a `DeString` that is not a valid UTF-8 string.
    ///
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let mut destring = DeString::from("hello");
    /// let devec = unsafe { destring.as_mut_devec() };
    /// devec.push_back(b' ');
    /// devec.push_back(b'w');
    /// devec.push_back(b'o');
    /// devec.push_back(b'r');
    /// devec.push_back(b'l');
    /// devec.push_back(b'd');
    /// assert_eq!(destring.as_str(), "hello world");
    /// ```
    #[inline]
    pub unsafe fn as_mut_devec(&mut self) -> &mut DeVec<u8> {
        &mut self.devec
    }

    /// Mutates the `DeString` by removing leading whitespace. The double-ended nature of the data structure means this can be done in-place.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let mut destring = DeString::from("  hello");
    /// let original_ptr = destring.as_ptr();
    /// destring.mut_trim_front();
    /// let new_ptr = destring.as_ptr();
    /// assert_eq!(destring.as_str(), "hello");
    /// assert!(std::ptr::eq(original_ptr, new_ptr));
    /// ```
    #[inline]
    pub fn mut_trim_front(&mut self) {
        let new_str = self.as_str().trim_start();
        let new_start = self.len() - new_str.len();
        // TODO: does this get properly inlined so that it becomes a single operation that changes the boundaries?
        //  If it does not, then we should do that truncation directly
        let drain = self.devec.drain(0..new_start);
        drop(drain);
    }

    /// Mutates the `DeString` by removing trailing whitespace.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let mut destring = DeString::from("hello  ");
    /// let original_ptr = destring.as_ptr();
    /// destring.mut_trim_back();
    /// let new_ptr = destring.as_ptr();
    /// assert_eq!(destring.as_str(), "hello");
    /// assert!(std::ptr::eq(original_ptr, new_ptr));
    /// ```
    #[inline]
    pub fn mut_trim_back(&mut self) {
        let new_str = self.as_str().trim_end();
        let new_end = new_str.len();
        // TODO: does this get properly inlined so that it becomes a single operation that changes the boundaries?
        //  If it does not, then we should do that truncation directly
        let drain = self.devec.drain(new_end..self.len());
        drop(drain);
    }

    /// Mutates the `DeString` by removing leading and trailing whitespace.
    /// The double-ended nature of the data structure means this can be done in-place.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let mut destring = DeString::from("  hello  ");
    /// let original_ptr = destring.as_ptr();
    /// destring.mut_trim();
    /// let new_ptr = destring.as_ptr();
    /// assert_eq!(destring.as_str(), "hello");
    /// assert!(std::ptr::eq(original_ptr, new_ptr));
    /// ```
    #[inline]
    pub fn mut_trim(&mut self) {
        self.mut_trim_front();
        self.mut_trim_back();
    }

    /// Parses the `DeString` as a UTF-8 string. This returns `Err` if the [`DeVec`] input's bytes do not form a valid UTF-8 string.
    /// # Examples
    /// ```
    /// # use spaghetto::{DeVec, DeString};
    /// let devec = DeVec::from(vec![104, 101, 108, 108, 111]);
    /// let destring = DeString::from_utf8(devec);
    /// assert_eq!(destring.unwrap().as_str(), "hello");
    /// ```
    /// ```should_panic
    /// # use spaghetto::{DeVec, DeString};
    /// let devec = DeVec::from(vec![104, 101, 108, 108, 111, 255]);
    /// let destring = DeString::from_utf8(devec);
    /// destring.expect("this should panic");
    /// ```
    #[inline]
    pub fn from_utf8(devec: DeVec<u8>) -> Result<Self, std::str::Utf8Error> {
        let _ = std::str::from_utf8(&devec)?;
        Ok(DeString { devec })
    }

    /// Reinterprets a DeVec as a UTF-8 string. This is unsafe because it does not check if the DeVec input is a valid UTF-8 string.
    /// # Safety
    /// The caller is responsible for ensuring that the DeVec input is a valid UTF-8 string.
    /// # Examples
    /// ```
    /// # use spaghetto::{DeVec, DeString};
    /// let devec = DeVec::from(vec![104, 101, 108, 108, 111]);
    /// let destring = unsafe { DeString::from_utf8_unchecked(devec) };
    /// assert_eq!(destring.as_str(), "hello");
    /// ```
    #[inline]
    pub unsafe fn from_utf8_unchecked(devec: DeVec<u8>) -> Self {
        DeString { devec }
    }

    /// Removes a range of bytes from the DeString and returns an iterator over the chars in the removed region.
    /// # Examples
    /// ```
    /// # use spaghetto::DeString;
    /// let mut destring = DeString::from("hello world");
    /// let mut drain = destring.drain(6..);
    /// let drained_values = drain.collect::<String>();
    /// assert_eq!(drained_values.as_str(), "world");
    /// assert_eq!(destring.as_str(), "hello ");
    /// ```
    #[inline]
    pub fn drain<R>(&mut self, range: R) -> destring_drain::Drain<'_>
    where
        R: std::ops::RangeBounds<usize>,
    {
        let start = match range.start_bound() {
            std::ops::Bound::Included(&start) => start,
            std::ops::Bound::Excluded(&start) => start + 1,
            std::ops::Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            std::ops::Bound::Included(&end) => end + 1,
            std::ops::Bound::Excluded(&end) => end,
            std::ops::Bound::Unbounded => self.len(),
        };

        // this will cause us to panic if the slice to be removed is not along a boundaries
        let _assert_valid_middle = &self.as_str()[start..end];
        Drain {
            inner_drain: crate::devec::Drain {
                devec: &mut self.devec,
                start,
                current: start,
                target: end,
            },
        }
    }
}

impl Default for DeString {
    #[inline]
    fn default() -> Self {
        DeString::new()
    }
}

impl std::fmt::Debug for DeString {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl std::fmt::Write for DeString {
    #[inline]
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.push_str_back(s);
        Ok(())
    }

    #[inline]
    fn write_char(&mut self, c: char) -> std::fmt::Result {
        self.push_char_back(c);
        Ok(())
    }
}

impl std::fmt::Display for DeString {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_str(), f)
    }
}

impl std::ops::Deref for DeString {
    type Target = str;
    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl std::ops::DerefMut for DeString {
    #[inline]
    fn deref_mut(&mut self) -> &mut str {
        self.mut_trim_back();
        self.as_mut_str()
    }
}

impl std::ops::Add<&str> for DeString {
    type Output = DeString;

    #[inline]
    fn add(mut self, rhs: &str) -> Self::Output {
        self.push_str_back(rhs);
        self
    }
}

impl std::ops::AddAssign<&str> for DeString {
    #[inline]
    fn add_assign(&mut self, rhs: &str) {
        self.push_str_back(rhs);
    }
}

impl<R> std::ops::Index<R> for DeString
where
    R: std::slice::SliceIndex<str>,
{
    type Output = <str as std::ops::Index<R>>::Output;
    #[inline]
    fn index(&self, index: R) -> &Self::Output {
        self.as_str().index(index)
    }
}

impl std::borrow::Borrow<str> for DeString {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl std::borrow::BorrowMut<str> for DeString {
    #[inline]
    fn borrow_mut(&mut self) -> &mut str {
        self.mut_trim_back();
        self.as_mut_str()
    }
}

impl std::borrow::Borrow<std::path::Path> for DeString {
    #[inline]
    fn borrow(&self) -> &std::path::Path {
        std::path::Path::new(self.as_str())
    }
}

impl From<&str> for DeString {
    #[inline]
    fn from(s: &str) -> Self {
        let mut destring = DeString::with_capacity(s.len());
        destring.devec.start = 0;
        destring.push_str_back(s);
        destring
    }
}

impl From<String> for DeString {
    #[inline]
    fn from(s: String) -> Self {
        // SAFETY: we know that the original buffer came from a String which is guaranteed to be valid UTF-8
        unsafe { Self::from_utf8_unchecked(s.into_bytes().into()) }
    }
}

impl<'a> FromIterator<DeString> for std::borrow::Cow<'a, str> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = DeString>>(iter: I) -> Self {
        std::borrow::Cow::Owned(iter.into_iter().collect())
    }
}

impl FromIterator<DeString> for String {
    #[inline]
    fn from_iter<I: IntoIterator<Item = DeString>>(iter: I) -> Self {
        let mut string = String::new();
        for s in iter {
            // no way to re-use s's buffer
            string += &s;
        }
        string
    }
}

impl FromIterator<String> for DeString {
    #[inline]
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        iter.into_iter().collect::<String>().into()
    }
}

impl FromIterator<DeString> for DeString {
    #[inline]
    fn from_iter<I: IntoIterator<Item = DeString>>(iter: I) -> Self {
        let mut destring = DeString::new();
        for s in iter {
            // no way to re-use s's buffer
            destring += &s;
        }
        destring
    }
}

impl FromIterator<char> for DeString {
    #[inline]
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        let s: String = iter.into_iter().collect();
        DeString::from(s)
    }
}

impl<'a> FromIterator<&'a char> for DeString {
    #[inline]
    fn from_iter<I: IntoIterator<Item = &'a char>>(iter: I) -> Self {
        iter.into_iter().copied().collect()
    }
}

impl std::str::FromStr for DeString {
    type Err = std::string::ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(DeString::from(s))
    }
}

impl std::convert::From<DeString> for String {
    #[inline]
    fn from(value: DeString) -> String {
        // value.as_str().to_string()
        // SAFETY: we know that the underlying DeVec has the correct length for the bytes we want to copy
        // copy the overlapping bytes from the inner DeVec to the front of its buffer
        let raw_parts: (*mut u8, usize, usize, usize) = value.devec.into_raw_parts();
        let (ptr, start, len, cap) = raw_parts;
        let start_of_buf = ptr;
        let start_of_valid_region = unsafe { ptr.add(start) };
        let length_to_copy = len;
        // write bytes
        unsafe {
            std::ptr::copy(start_of_valid_region, start_of_buf, length_to_copy);
        }
        // create a new String from the raw parts
        // TODO: we copied valid utf8 bytes, and we know that the allocation was originally made for with a cap of `cap` and an item type of `u8`
        unsafe { String::from_raw_parts(ptr, len, cap) }
    }
}

impl std::convert::From<DeString> for std::path::PathBuf {
    #[inline]
    fn from(value: DeString) -> std::path::PathBuf {
        String::from(value).into()
    }
}

impl PartialEq for DeString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<'a> PartialEq<&'a str> for DeString {
    #[inline]
    fn eq(&self, other: &&'a str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<str> for DeString {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<String> for DeString {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<DeString> for str {
    #[inline]
    fn eq(&self, other: &DeString) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<DeString> for String {
    #[inline]
    fn eq(&self, other: &DeString) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<'a> PartialEq<DeString> for &'a str {
    #[inline]
    fn eq(&self, other: &DeString) -> bool {
        *self == other.as_str()
    }
}

impl Eq for DeString {}

impl PartialOrd for DeString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialOrd<str> for DeString {
    #[inline]
    fn partial_cmp(&self, other: &str) -> Option<std::cmp::Ordering> {
        self.as_str().partial_cmp(other)
    }
}

impl PartialOrd<String> for DeString {
    #[inline]
    fn partial_cmp(&self, other: &String) -> Option<std::cmp::Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl Ord for DeString {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl std::hash::Hash for DeString {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}
pub use destring_drain::Drain;
pub(crate) mod destring_drain {
    /// An iterator over the chars in a [`DeString`](super::DeString) that removes them from the `DeString`.
    /// This can be obtained from the [`DeString::drain`](super::DeString::drain) method.
    pub struct Drain<'a> {
        pub(crate) inner_drain: crate::devec::Drain<'a, u8, crate::FrontToBack, crate::Middle>,
    }

    impl std::iter::Iterator for Drain<'_> {
        type Item = char;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            // SAFETY: we know that the `DeVec<u8>` buffer is a valid UTF-8 string because this must have come from a `DeString`
            let inner_drain_start = self.inner_drain.current;
            let inner_drain_end = self.inner_drain.target;
            let inner_str = unsafe {
                std::str::from_utf8_unchecked(
                    &self.inner_drain.devec[inner_drain_start..inner_drain_end],
                )
            };
            let mut char_indices = inner_str.char_indices();
            let (_, next_char) = char_indices.next()?;
            let char_utf8_len = next_char.len_utf8();
            // this should use advance_by, but instead we have to use next because advance_by isn't stable
            for _ in 0..char_utf8_len {
                self.inner_drain.next();
            }
            Some(next_char)
        }

        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            let remaining_bytes = self.inner_drain.target - self.inner_drain.current;
            // every char is at least 1 byte and up to 4 bytes
            ((remaining_bytes + 3) / 4, Some(remaining_bytes))
        }
    }

    impl std::iter::FusedIterator for Drain<'_> {}
}

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
#[doc(hidden)]
pub(crate) mod serde_impls {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    impl Serialize for DeString {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            self.as_str().serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for DeString {
        fn deserialize<D>(deserializer: D) -> Result<DeString, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            Ok(DeString::from(s))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn push_char_order() {
        let mut destring = DeString::new();
        destring.push_char_back('a');
        destring.push_char_back('b');
        destring.push_char_back('c');
        assert_eq!(destring.as_str(), "abc");
    }

    #[test]
    pub fn pop_char_order() {
        let mut destring = DeString::new();
        destring.push_char_back('a');
        destring.push_char_back('b');
        destring.push_char_back('c');
        assert_eq!(destring.pop_back(), Some('c'));
        assert_eq!(destring.pop_back(), Some('b'));
        assert_eq!(destring.pop_back(), Some('a'));
        assert_eq!(destring.pop_back(), None);
    }

    #[test]
    pub fn push_str_order() {
        let mut destring = DeString::new();
        destring.push_str_back("abc");
        destring.push_str_back("def");
        destring.push_str_back("ghi");
        assert_eq!(destring.as_str(), "abcdefghi");
    }

    #[test]
    pub fn pop_str_order() {
        let mut destring = DeString::new();
        destring.push_str_back("abc");
        destring.push_str_back("def");
        destring.push_str_back("ghi");
        assert_eq!(destring.pop_back(), Some('i'));
        assert_eq!(destring.pop_back(), Some('h'));
        assert_eq!(destring.pop_back(), Some('g'));
        assert_eq!(destring.pop_back(), Some('f'));
        assert_eq!(destring.pop_back(), Some('e'));
        assert_eq!(destring.pop_back(), Some('d'));
        assert_eq!(destring.pop_back(), Some('c'));
        assert_eq!(destring.pop_back(), Some('b'));
        assert_eq!(destring.pop_back(), Some('a'));
        assert_eq!(destring.pop_back(), None);
    }

    #[test]
    pub fn push_str_front_order() {
        let mut destring = DeString::new();
        destring.push_str_front("abc");
        destring.push_str_front("def");
        destring.push_str_front("ghi");
        assert_eq!(destring.as_str(), "ghidefabc");
    }

    #[test]
    pub fn pop_str_front_order() {
        let mut destring = DeString::new();
        destring.push_str_front("abc");
        destring.push_str_front("def");
        destring.push_str_front("ghi");
        assert_eq!(destring.as_str(), "ghidefabc");
        assert_eq!(destring.pop_back(), Some('c'));
        assert_eq!(destring.pop_back(), Some('b'));
        assert_eq!(destring.pop_back(), Some('a'));
        assert_eq!(destring.pop_back(), Some('f'));
        assert_eq!(destring.pop_back(), Some('e'));
        assert_eq!(destring.pop_back(), Some('d'));
        assert_eq!(destring.pop_back(), Some('i'));
        assert_eq!(destring.pop_back(), Some('h'));
        assert_eq!(destring.pop_back(), Some('g'));
        assert_eq!(destring.pop_back(), None);
    }

    #[test]
    pub fn test_trim_front() {
        let mut destring = DeString::new();
        destring.push_str_back("   abc");
        destring.mut_trim_front();
        assert_eq!(destring.as_str(), "abc");
        destring.push_char_front(' ');
        destring.mut_trim_front();
        assert_eq!(destring.as_str(), "abc");
    }

    #[test]
    pub fn test_trim_back() {
        let mut destring = DeString::new();
        destring.push_str_back("abc   ");
        destring.mut_trim_back();
        assert_eq!(destring.as_str(), "abc");
        destring.push_char_back(' ');
        destring.mut_trim_back();
        assert_eq!(destring.as_str(), "abc");
    }
}
