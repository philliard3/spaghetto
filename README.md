# Spaghetto

Spaghetto is a library for creating double-ended data structures that can be rolled out or eaten on from either side.
This includes the base structure `DeVec`, a double-ended `Vec` (heap-allocated dynamically sized array-like collection) that can be used as a deque, and `DeString`, a double-ended alternative to `String`.
<br>
## Examples

### DeVec
A `DeVec` (Double-Ended Vec) can be used as a double-ended queue, but with the added benefit of being able to get a single contiguous slice of the entire structure.
```rust
use spaghetto::DeVec;
let mut devec = DeVec::new();
devec.push_back(1);
devec.push_front(2);
devec.push_back(3);
assert_eq!(devec.pop_front(), Some(2));
// now let's overload one side to force it to reallocate. With a VecDeque this would no longer be a contiguous allocation.
for i in 0..100 {
   devec.push_back(i);
}

// we can get a single contiguous slice of the entire DeVec without having to shift elements.
let slice = devec.as_slice();
```

### DeString
A `DeString` can be used as a double-ended string, and because of this, we can efficiently remove extra whitespace from either side, mutating in place and maintining a single contiguous string slice without the cost of shifting like with a String.
```rust
use spaghetto::DeString;
let mut destring = DeString::from("  hello world  ");
destring.mut_trim_front();
// It is contiguous and also mutated its starting position!
assert_eq!(destring.as_str(), "hello world  ");
```
