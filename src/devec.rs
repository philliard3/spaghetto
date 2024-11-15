//! A [`DeVec`] is a double-ended vector that is meant to handle easy insertions and deletions from either side.
//! Its API is similar to [`VecDeque`](std::collections::VecDeque), but with some conveniences from [`Vec`], both because of its unique properties and because we can see the nightly implementations before they are stabilized.
//! This data structure has benefits over a VecDeque in that it can maintain a contiguous allocation for its entire life.
//! This means that it can easily be used with functions that require a contiguous slice of memory.

use crate::{BackToFront, DropBehavior, FrontToBack, Middle, RebalanceBehavior, RebalanceStrategy};
use std::alloc::Layout;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

/// A double-ended vector that allows for efficient insertion and removal at both ends.
/// The DeVec is backed by a buffer that is dynamically resized as needed.
/// Capacity is doubled on attempts to push at either edge of the buffer.
/// DeVec will not allocate memory for ZSTs.
///
/// # Examples
/// ```
/// use spaghetto::DeVec;
/// let mut devec = DeVec::new();
/// devec.push_back(2);
/// devec.push_front(1);
/// devec.push_back(3);
/// assert_eq!(&*devec, &[1, 2, 3]);
/// ```
///
/// # Drop Order
/// The default drop order is front-to-back, meaning that elements are dropped in the same order as iteration would happen.
/// This can be changed to back-to-front by using the [`BackToFront`] drop order.
///
/// # Rebalance Behavior
/// The default rebalance behavior ([`Middle`])  is to center the data so that the middle of the slice is at the middle of the buffer after the buffer grows.
/// This can be changed to do one of the following:
/// - [`StartAtFront`](crate::settings::StartAtFront) will always make the slice start at the front of the buffer when growing. This will make the behavior very similar to a [`Vec`].
/// - [`FavorCrowdedSide`](crate::settings::FavorCrowdedSide) will favor the side of the buffer that has less room when growing. This will benefit workflows that push more to one side than another, but in workflows that push to both sides equally, it will have to grow each side roughly half the time.
/// - [`OnlyChangeCrowdedSide`](crate::settings::OnlyChangeCrowdedSide) will only grow the side of the buffer that has no room. Like favoring the crowded side, this works better for workloads that push more often to one side, but it is more likely to punish access patterns that are more evenly distributed.
pub struct DeVec<T, DropOrder = FrontToBack, Rebalance = Middle>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    pub(crate) ptr: NonNull<T>,
    pub(crate) start: usize,
    pub(crate) cap: usize,
    pub(crate) len: usize,
    pub(crate) drop_order: DropOrder,
    pub(crate) rebalance: Rebalance,
}

impl<T: Debug, DropOrder, Rebalance> Debug for DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.as_slice(), f)
    }
}

// debug representation is allowed to not be used elsewhere
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct DeVecDebug<T, DropOrder> {
    pub(crate) ptr: NonNull<T>,
    pub(crate) start: usize,
    pub(crate) cap: usize,
    pub(crate) len: usize,
    pub(crate) drop_order: DropOrder,
}

impl<T, DropOrder, Rebalance> DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[allow(dead_code)]
    pub(crate) fn debug(&self) -> DeVecDebug<T, DropOrder> {
        DeVecDebug {
            ptr: self.ptr,
            start: self.start,
            cap: self.cap,
            len: self.len,
            drop_order: self.drop_order,
        }
    }
}

unsafe impl<T: Send, DropOrder, Rebalance> Send for DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
}
unsafe impl<T: Sync, DropOrder, Rebalance> Sync for DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
}

impl<T> DeVec<T, FrontToBack> {
    /// Creates a new, empty `DeVec`. The default is to drop items from front to back.
    /// A double ended queue does not necessarily preserve insertion order, so the drop order might matter for some use cases.
    /// If you need to drop items from back to front, use [`DeVec::new_with_drop_order::<BackToFront>()`](DeVec::new_with_drop_order).
    ///
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// assert_eq!(&*devec, &[1, 2]);
    /// ```
    #[inline]
    pub const fn new() -> Self {
        let cap = if std::mem::size_of::<T>() == 0 {
            usize::MAX
        } else {
            0
        };
        DeVec {
            ptr: NonNull::dangling(),
            start: 0,
            len: 0,
            cap,
            drop_order: FrontToBack,
            rebalance: Middle,
        }
    }

    /// Inverts the drop order of the `DeVec` to back-to-front.
    /// This is useful when you need to drop items in the reverse order of insertion.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// let devec = devec.as_back_to_front();
    /// assert_eq!(&*devec, &[1, 2]);
    /// ```
    #[inline]
    #[must_use = "This DeVec's drop order has been changed. Please make sure to use the new DeVec or drop it explicitly."]
    pub fn as_back_to_front(self) -> DeVec<T, BackToFront> {
        let this = std::mem::ManuallyDrop::new(self);
        DeVec {
            ptr: this.ptr,
            start: this.start,
            len: this.len,
            cap: this.cap,
            drop_order: BackToFront,
            rebalance: this.rebalance,
        }
    }

    #[inline]
    #[must_use = "This DeVec's rebalance behavior has been changed. Please make sure to use the new DeVec or drop it explicitly."]
    pub fn as_middle(self) -> DeVec<T, FrontToBack, Middle> {
        let this = std::mem::ManuallyDrop::new(self);
        DeVec {
            ptr: this.ptr,
            start: this.start,
            len: this.len,
            cap: this.cap,
            drop_order: this.drop_order,
            rebalance: Middle,
        }
    }
}

impl<T, DropOrder, Rebalance> DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    /// Creates a new, empty `DeVec` with a specified drop order.
    /// A double ended queue does not necessarily preserve insertion order, so the drop order might matter for some use cases.
    ///
    /// # Examples
    /// ```
    /// use spaghetto::{DeVec, BackToFront};
    /// let mut devec = DeVec::<_>::new_with_drop_order::<BackToFront>();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// assert_eq!(&*devec, &[1, 2]);
    /// ```
    #[inline]
    #[must_use]
    pub fn new_with_drop_order<D>() -> DeVec<T, D>
    where
        D: DropBehavior + Default,
    {
        let cap = if std::mem::size_of::<T>() == 0 {
            usize::MAX
        } else {
            0
        };
        DeVec {
            ptr: NonNull::dangling(),
            start: 0,
            len: 0,
            cap,
            drop_order: Default::default(),
            rebalance: Default::default(),
        }
    }

    #[inline]
    #[must_use]
    pub fn new_with_rebalance_behavior<R>() -> DeVec<T, DropOrder, R>
    where
        R: RebalanceBehavior + Default,
    {
        let cap = if std::mem::size_of::<T>() == 0 {
            usize::MAX
        } else {
            0
        };
        DeVec {
            ptr: NonNull::dangling(),
            start: 0,
            len: 0,
            cap,
            drop_order: Default::default(),
            rebalance: Default::default(),
        }
    }

    /// Creates a new, empty `DeVec` with a specified capacity.
    /// The DeVec will be able to hold at least `cap` elements without reallocating.
    /// If `T` is a zero-sized type, the capacity is set to [`usize::MAX`].
    /// # Panics
    /// Panics if the allocation size exceeds [`isize::MAX`]
    /// Other failure happens if an allocation error occurs.
    /// # Examples
    /// ```
    /// use spaghetto::DeVec;
    /// let mut devec : DeVec<i32> = DeVec::with_capacity(10);
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// assert!(devec.capacity() >= 10);
    /// ```
    /// ```
    /// use spaghetto::DeVec;
    /// let mut devec = DeVec::<()>::with_capacity(10);
    /// devec.push_back(());
    /// devec.push_back(());
    /// assert!(devec.capacity() >= 10);
    /// ```
    #[inline]
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        let (cap, ptr) = if std::mem::size_of::<T>() == 0 {
            (usize::MAX, NonNull::dangling())
        } else {
            let layout = Layout::array::<T>(cap).unwrap();
            let ptr = unsafe { std::alloc::alloc(layout) };
            let ptr = match NonNull::new(ptr as *mut T) {
                Some(p) => p,
                None => std::alloc::handle_alloc_error(layout),
            };
            (cap, ptr)
        };
        let start = if cap == 0 { 0 } else { cap / 2 };
        DeVec {
            ptr,
            start,
            len: 0,
            cap,
            drop_order: Default::default(),
            rebalance: Default::default(),
        }
    }

    /// Creates a new, empty `DeVec` with a specified capacity and drop order.
    /// The DeVec will be able to hold at least `cap` elements without reallocating.
    ///
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// use spaghetto::BackToFront;
    /// let mut devec = DeVec::<_>::with_capacity_and_drop_order::<BackToFront>(10);
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// assert_eq!(&*devec, &[1, 2]);
    /// ```
    #[inline]
    #[must_use]
    pub fn with_capacity_and_drop_order<D>(cap: usize) -> DeVec<T, D>
    where
        D: DropBehavior + Default,
    {
        DeVec::<T, D>::with_capacity(cap)
    }

    /// Changes the drop order of the `DeVec` to the specified order in-place.
    ///
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// use spaghetto::BackToFront;
    /// let mut devec = DeVec::<_>::new();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// let devec = devec.with_drop_order::<BackToFront>();
    /// assert_eq!(&*devec, &[1, 2]);
    /// ```
    #[inline]
    #[must_use]
    pub fn with_drop_order<D>(self) -> DeVec<T, D, Rebalance>
    where
        D: DropBehavior,
    {
        let this = std::mem::ManuallyDrop::new(self);
        DeVec {
            ptr: this.ptr,
            start: this.start,
            len: this.len,
            cap: this.cap,
            drop_order: Default::default(),
            rebalance: this.rebalance,
        }
    }

    /// Changes the rebalance behavior of the `DeVec` to the specified behavior in-place.
    ///
    #[inline]
    #[must_use]
    pub fn with_rebalance_behavior<R>(self) -> DeVec<T, DropOrder, R>
    where
        R: RebalanceBehavior,
    {
        let this = std::mem::ManuallyDrop::new(self);
        DeVec {
            ptr: this.ptr,
            start: this.start,
            len: this.len,
            cap: this.cap,
            drop_order: this.drop_order,
            rebalance: Default::default(),
        }
    }
}

impl<T, DropOrder, Rebalance> DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    // Grows the vector by doubling the capacity.
    // Currently it also shifts elements slightly to the side that was lopsided. In the event of a tie it chooses the start of the buffer.
    #[inline]
    pub(crate) fn grow(&mut self) {
        // since we set the capacity to `usize::MAX` when T has size 0,
        // getting to here necessarily means the Vec is overfull.
        assert!(std::mem::size_of::<T>() != 0, "capacity overflow");

        let (new_cap, new_start, new_layout) = if self.cap == 0 {
            let starting_cap = 5;
            let midpoint = starting_cap / 2;
            (
                starting_cap,
                midpoint,
                Layout::array::<T>(starting_cap).unwrap(),
            )
        } else {
            // This can't overflow since self.cap <= isize::MAX.
            let new_cap = 2 * self.cap;

            // `Layout::array` checks that the number of bytes is <= usize::MAX,
            // but this is redundant since old_layout.size() <= isize::MAX,
            // so the `unwrap` should never fail.
            let new_layout = Layout::array::<T>(new_cap).unwrap();

            // TODO: optimize further by adjusting primarily the side that is lopsided

            let nc = new_cap as f32;
            let len = self.len as f32;

            let new_start: usize =
                match <Rebalance as crate::settings::seal_rebalance_behavior::Sealed>::BEHAVIOR {
                    RebalanceStrategy::StartAtFront => 0,
                    RebalanceStrategy::Middle => {
                        let midpoint = new_cap / 2;
                        midpoint - (self.len / 2)
                    }
                    RebalanceStrategy::FavorCrowdedSide => {
                        // find out which side has no room, favoring the back side if it's a tie
                        let back_space = self.space_back();
                        if back_space == 0 {
                            let new_offset = 0.2 * nc;
                            // now there's much more room at the back, and a little more at the front
                            new_offset as usize
                        } else {
                            let new_end = 0.8 * nc;
                            let new_start = new_end - len;
                            new_start as usize
                        }
                    }
                    RebalanceStrategy::OnlyChangeCrowdedSide => {
                        // find out which side has no room, favoring the back side if it's a tie
                        let back_space = self.space_back();
                        if back_space == 0 {
                            // we know there's no room at the back, so we keep the starting offset the same
                            self.start
                        } else {
                            // back space is kept the same, so front space is increased by the difference between that and the new len and capacity
                            new_cap - (self.len + back_space)
                        }
                    }
                };

            /*
            // push from the left
            let mut new_start = if self.start == 0 {
                // this new offset should be the midpoint of the new buffer
                let new_offset = 0.6 * nc;
                let half_len = len * 0.5;
                let new_start = (new_offset.max(half_len) - half_len) as usize;
                new_start
            } else {
                // push from the right
                let new_offset = 0.4 * nc;
                let half_len = len * 0.5;
                let new_start = (new_offset.max(half_len) - half_len) as usize;
                new_start
            };

            if new_start == 0 || new_start + self.len >= new_cap {
                let midpoint = new_cap / 2;
                new_start = midpoint - (self.len / 2);
            }
            */

            (new_cap, new_start, new_layout)
        };

        // Ensure that the new allocation doesn't exceed `isize::MAX` bytes.
        assert!(
            new_layout.size() <= isize::MAX as usize,
            "Allocation too large"
        );

        let new_ptr = if self.cap == 0 {
            unsafe { std::alloc::alloc(new_layout) }
        } else {
            let old_layout = Layout::array::<T>(self.cap).unwrap();
            let old_ptr = self.ptr.as_ptr() as *mut u8;
            unsafe { std::alloc::realloc(old_ptr, old_layout, new_layout.size()) }
        };

        // If allocation fails, `new_ptr` will be null, in which case we abort.
        self.ptr = match NonNull::new(new_ptr as *mut T) {
            Some(p) => p,
            None => std::alloc::handle_alloc_error(new_layout),
        };

        if self.start != new_start {
            unsafe {
                let old_start_ptr = self.ptr.as_ptr().add(self.start);
                let new_start_ptr = self.ptr.as_ptr().add(new_start);
                std::ptr::copy(old_start_ptr, new_start_ptr, self.len);
            }
            self.start = new_start;
        }

        self.cap = new_cap;
    }

    /// Pushes an element to the back of the `DeVec`'s buffer.
    ///
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut vec: DeVec<i32> = DeVec::from([42, 10]);
    /// vec.push_back(100);
    /// assert_eq!(vec.pop_back(), Some(100));
    /// ```
    #[inline]
    pub fn push_back(&mut self, elem: T) {
        if std::mem::size_of::<T>() == 0 {
            self.len += 1;
            return;
        }

        while self.len + self.start >= self.cap {
            // if we're at the end of the buffer
            // if self.len == self.cap {
            self.grow();
        }

        unsafe {
            // "semantically, [the element] is moved" to the new pointer location
            std::ptr::write(self.ptr.as_ptr().add(self.start + self.len), elem);
        }

        // Can't fail, we'll OOM first.
        // TODO: make try_ versions of all methods to account for OOM
        self.len += 1;
    }

    /// Pushes an element to the front of the `DeVec`'s buffer.
    ///
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut vec: DeVec<i32> = DeVec::from([42, 10]);
    /// vec.push_front(100);
    /// assert_eq!(vec.pop_front(), Some(100));
    /// ```
    #[inline]
    pub fn push_front(&mut self, elem: T) {
        if std::mem::size_of::<T>() == 0 {
            self.len += 1;
            return;
        }
        // if we're at the start of the buffer
        while self.start == 0 {
            self.grow();
        }

        unsafe {
            // "semantically, [the element] is moved" to the new pointer location
            if self.cap == 1 {
                std::ptr::write(self.ptr.as_ptr(), elem);
            } else {
                // it should be impossible for self.start to be 0 so subtracting 1 is fine
                std::ptr::write(self.ptr.as_ptr().add(self.start - 1), elem);

                // Can't fail, we'll OOM first.
                // TODO: make try_ versions of all methods to account for OOM
                self.len += 1;
                self.start -= 1;
            }
        }
    }

    /// Pops an element from the back of the `DeVec`'s buffer, returning `None` if the buffer is empty.
    ///
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut vec: DeVec<i32> = DeVec::from([42, 10]);
    /// assert_eq!(vec.pop_back(), Some(10));
    /// assert_eq!(vec.pop_back(), Some(42));
    /// assert_eq!(vec.pop_back(), None);
    /// ```
    #[inline]
    pub fn pop_back(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;

            if std::mem::size_of::<T>() == 0 {
                unsafe {
                    return Some(std::ptr::read(self.ptr.as_ptr()));
                }
            }

            unsafe { Some(std::ptr::read(self.ptr.as_ptr().add(self.start + self.len))) }
        }
    }

    /// Pops an element from the front of the `DeVec`'s buffer, returning `None` if the buffer is empty.
    ///
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut vec: DeVec<i32> = DeVec::from([42, 10]);
    /// assert_eq!(vec.pop_front(), Some(42));
    /// assert_eq!(vec.pop_front(), Some(10));
    /// assert_eq!(vec.pop_front(), None);
    /// ```
    #[inline]
    pub fn pop_front(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            if std::mem::size_of::<T>() == 0 {
                self.len -= 1;
                unsafe {
                    return Some(std::ptr::read(self.ptr.as_ptr()));
                }
            }
            let ret = unsafe {
                //
                // so it should be self.ptr.as_ptr().add(self.len + self.start)
                Some(std::ptr::read(self.ptr.as_ptr().add(self.start)))
            };
            self.len -= 1;
            self.start += 1;
            ret
        }
    }

    /// Returns the number of elements in the `DeVec`.
    ///
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut vec: DeVec<i32> = DeVec::with_capacity(10);
    /// vec.push_back(42);
    /// assert!(vec.capacity() >= 10);
    /// assert_eq!(vec.len(), 1);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the total capacity of the `DeVec`. This is less than or equal to the number of elements that it can hold.
    /// A DeVec tends to re-allocate with space on either side, so there is less room on either side than in a [`Vec`] with the same capacity.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.cap
    }

    /// Returns the number of elements the `DeVec` can hold without reallocating.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut vec: DeVec<i32> = DeVec::with_capacity(10);
    /// vec.push_back(42);
    /// assert!(vec.capacity() >= 10);
    /// assert_eq!(vec.len(), 1);
    /// assert!(vec.space_front() > 1);
    /// assert!(vec.space_front() < vec.capacity());
    /// ```
    #[inline]
    pub fn space_front(&self) -> usize {
        self.start
    }

    /// Returns the number of elements the `DeVec` can hold without reallocating.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut vec: DeVec<i32> = DeVec::with_capacity(10);
    /// vec.push_back(42);
    /// assert!(vec.capacity() >= 10);
    /// assert_eq!(vec.len(), 1);
    /// assert!(vec.space_back() > 1);
    /// assert!(vec.space_back() < vec.capacity());
    /// ```
    #[inline]
    pub fn space_back(&self) -> usize {
        self.cap - (self.start + self.len)
    }

    /// Returns the starting offset of the `DeVec`'s buffer.
    /// This is the number of elements that are stored before the first element in the buffer.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut vec: DeVec<i32> = DeVec::with_capacity(10);
    /// vec.push_back(42);
    /// assert!(vec.capacity() >= 10);
    /// assert_eq!(vec.len(), 1);
    /// assert!(vec.starting_offset() > 0);
    /// ```
    #[inline]
    pub fn starting_offset(&self) -> usize {
        self.start
    }

    /// Returns true if the vector contains no elements.
    ///
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut v = DeVec::new();
    /// assert!(v.is_empty());
    ///
    /// v.push_back(1);
    /// assert!(!v.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Removes all elements from the `DeVec` and drops them in the order specified by the DeVec's drop order.
    /// Note that this method has no effect on the allocated capacity of the DeVec.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// devec.clear();
    /// assert!(devec.is_empty());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        while (if DropOrder::IS_INVERTED {
            self.pop_back()
        } else {
            self.pop_front()
        })
        .is_some()
        {
            // drop the element
        }
    }

    /// Removes all elements from the `DeVec` and drops them in the order specified by the parameterized drop order.
    /// Note that this method has no effect on the allocated capacity of the DeVec.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// devec.clear_with_order(true);
    /// assert!(devec.is_empty());
    /// ```
    #[inline]
    pub fn clear_with_order(&mut self, drop_from_back: bool) {
        while (if drop_from_back {
            self.pop_back()
        } else {
            self.pop_front()
        })
        .is_some()
        {
            // drop the element
        }
    }

    ///Extracts a slice containing the entire vector.
    /// Equivalent to `&s[..]`.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// assert_eq!(devec.as_slice(), &[1, 2]);
    /// ```
    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        self
    }

    /// Extracts a mutable slice containing the entire vector.
    /// Equivalent to `&mut s[..]`.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// devec.as_mut_slice()[0] = 3;
    /// assert_eq!(devec.as_slice(), &[3, 2]);
    /// ```
    #[inline]
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self
    }

    /// Returns a raw pointer to the buffer. Note that this is the start of the allocation, not the start of the slice.
    /// # Safety
    /// The caller is responsible for upholding the integrity of the data structure when doing unsafe things with the pointer.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// let ptr = devec.as_ptr();
    /// let start = devec.starting_offset();
    /// unsafe {
    ///    assert_eq!(*ptr.add(start), 1);
    /// }
    /// ```
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    /// Returns a raw pointer to the buffer. Note that this is the start of the allocation, not the start of the slice.
    /// # Safety
    /// The caller is responsible for upholding the integrity of the data structure when doing unsafe things with the pointer.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// let ptr = devec.as_mut_ptr();
    /// let start = devec.starting_offset();
    /// unsafe {
    ///   *ptr.add(start) = 2;
    /// }
    /// ```
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Reserves capacity for at least `additional` more elements to be inserted in the back of the `DeVec`.
    /// The collection may reserve more space to avoid frequent reallocations.
    /// After calling `reserve_back`, capacity will be greater than or equal to `self.len() + additional`.
    ///
    /// # Panics
    /// Panics if the allocation size exceeds [`isize::MAX`]
    /// Other failure happens if an allocation error occurs.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// devec.reserve_back(10);
    /// assert!(devec.capacity() >= 12);
    /// ```
    // TODO: is OOM a panic?
    #[inline]
    pub fn reserve_back(&mut self, additional: usize) {
        while self.cap - (self.start + self.len) < additional {
            self.grow();
        }
    }

    /// Reserves capacity for at least `additional` more elements to be inserted in the front of the `DeVec`.
    /// The collection may reserve more space to avoid frequent reallocations.
    /// After calling `reserve_front`, capacity will be greater than or equal to `self.len() + additional`.
    /// Returns the new capacity.
    /// # Panics
    /// Panics if the allocation size exceeds [`isize::MAX`]
    /// Other failure happens if an allocation error occurs.
    /// # Examples
    /// ```
    /// use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// devec.reserve_front(10);
    /// assert!(devec.capacity() >= 12);
    /// ```
    #[inline]
    pub fn reserve_front(&mut self, additional: usize) {
        while self.start < additional {
            self.grow();
        }
    }

    /// Inserts an element at position index within the DeVec, shifting all elements after it to the right.
    ///
    /// # Panics
    /// Panics if `index > len`.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// devec.insert(1, 3);
    /// assert_eq!(devec.as_slice(), &[1, 3, 2]);
    /// ```
    /// ```should_panic
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// devec.insert(3, 3);
    /// ```
    #[inline]
    pub fn insert(&mut self, index: usize, elem: T) {
        assert!(index <= self.len, "index out of bounds");
        if index == self.len {
            self.push_back(elem);
            return;
        }
        if index == 0 {
            self.push_front(elem);
            return;
        }

        // basic reserve
        // shift all elements from that point backwards
        // let shift_back = true;
        let shift_back = index >= (self.len / 2);
        if shift_back {
            if self.space_back() < 1 {
                self.grow();
            }
            unsafe {
                let ptr = self.ptr.as_ptr().add(self.start + index);
                std::ptr::copy(ptr, ptr.add(1), self.len - index);
                std::ptr::write(ptr, elem);
            }
        } else {
            // shift elements to front
            if self.space_front() < 1 {
                self.grow();
            }
            unsafe {
                let ptr = self.ptr.as_ptr().add(self.start);
                // copy all data back by one
                std::ptr::copy(ptr, ptr.sub(1), index);
                std::ptr::write(ptr, elem);
            }
            self.start -= 1;
        }

        self.len += 1;
    }

    /// Removes and returns the element at position `index` within the `DeVec`, shifting all elements after it to the left.
    /// # Panics
    /// Panics if `index` is out of bounds.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// assert_eq!(devec.remove(1), 2);
    /// assert_eq!(devec.as_slice(), &[1]);
    /// ```
    #[inline]
    // TODO: should this be must_use?
    pub fn remove(&mut self, index: usize) -> T {
        // index is trivially >= 0 because it's a usize
        assert!(index < self.len, "index out of bounds");
        if index == 0 {
            return self.pop_front().unwrap();
        }
        if index == self.len - 1 {
            return self.pop_back().unwrap();
        }
        // TODO: smarter shifting based on index so we copy at most half the elements
        unsafe {
            let ptr = self.ptr.as_ptr().add(self.start + index);
            let result = std::ptr::read(ptr);
            std::ptr::copy(ptr.add(1), ptr, self.len - index - 1);
            self.len -= 1;
            result
        }
    }

    /// Swaps the specified element with the last one, then pops the last one.
    /// This will be faster than `remove` because it doesn't need to shift elements.
    /// If you would like to smartly remove by moving either the first or the last depending on the current balance of the `DeVec`, then use [`smart_swap_remove`](DeVec::smart_swap_remove).
    /// # Panics
    /// Panics if `index` is out of bounds.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// devec.swap_remove(0);
    /// assert_eq!(devec.as_slice(), &[2]);
    /// ```
    /// ```should_panic
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// devec.swap_remove(3);
    /// ```
    #[inline]
    pub fn swap_remove(&mut self, index: usize) -> T {
        assert!(index < self.len, "index out of bounds");
        // subtraction can never overflow because then index == 0 == len and that would already have panicked
        if index == self.len - 1 {
            return self.pop_back().unwrap();
        }
        let last_index = self.len - 1;
        self.as_mut_slice().swap(index, last_index);
        // this can never panic because otherwise index would be equal to len
        self.pop_back().unwrap()
    }

    /// Swaps the specified element with the first or last one, depending on which one is closer to the end of the buffer.
    /// for a simpler version that always swaps with the back like [`Vec::swap_remove`](Vec::swap_remove), use [`swap_remove`](DeVec::swap_remove).
    /// # Panics
    /// Panics if `index` is out of bounds.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// devec.smart_swap_remove(0);
    /// assert_eq!(devec.as_slice(), &[2]);
    /// ```
    /// ```should_panic
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// devec.smart_swap_remove(3);
    /// ```
    #[inline]
    pub fn smart_swap_remove(&mut self, index: usize) -> T {
        assert!(index < self.len, "index out of bounds");
        if index == 0 {
            return self.pop_front().unwrap();
        }
        if index == self.len - 1 {
            return self.pop_back().unwrap();
        }

        let front_space = self.space_front();
        let back_space = self.space_back();
        if front_space > back_space {
            // move the last to the back and pop it so the back gets more room
            let last_index = self.len - 1;
            self.as_mut_slice().swap(index, last_index);
            self.pop_back().unwrap()
        } else {
            // move the first element to the back
            let first_index = 0;
            self.as_mut_slice().swap(first_index, index);
            self.pop_front().unwrap()
        }
    }

    /// Internal method for draining a range of elements from the `DeVec`.
    #[inline]
    pub(crate) fn drain_inner(
        &mut self,
        range: std::ops::Range<usize>,
    ) -> Drain<'_, T, DropOrder, Rebalance> {
        let start = range.start;
        let end = range.end;
        assert!(start <= end, "start must be less than or equal to end");
        assert!(
            end <= self.len,
            "end must be less than or equal to the length of the DeVec"
        );
        let devec_start = self.start + start;
        let devec_target = self.start + end;
        Drain {
            devec: self,
            start: devec_start,
            current: devec_start,
            target: devec_target,
        }
    }

    /// Removes the elements specified by the range from the `DeVec` and returns an iterator over the removed elements.
    /// # Panics
    /// Panics if the range is out of bounds.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec : DeVec<i32> = DeVec::from([1, 2, 3, 4, 5]);
    /// let removed: Vec<_> = devec.drain(1..4).collect();
    /// assert_eq!(removed, [2, 3, 4]);
    /// assert_eq!(devec.as_slice(), &[1, 5]);
    /// ```
    #[inline]
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, T, DropOrder, Rebalance>
    where
        R: std::ops::RangeBounds<usize>,
    {
        let start = match range.start_bound() {
            std::ops::Bound::Included(&s) => s,
            std::ops::Bound::Excluded(&s) => s + 1,
            std::ops::Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            std::ops::Bound::Included(&e) => e + 1,
            std::ops::Bound::Excluded(&e) => e,
            std::ops::Bound::Unbounded => self.len,
        };
        self.drain_inner(start..end)
    }

    /// Removes the elements specified by the predicate function from the `DeVec` and returns an iterator over the removed elements.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec : DeVec<i32> = DeVec::from([1, 2, 3, 4, 5]);
    /// let removed: Vec<_> = devec.extract_if(|_, elem| *elem % 2 == 0).collect();
    /// assert_eq!(removed.as_slice(), &[2, 4]);
    /// assert_eq!(devec.as_slice(), &[1, 3, 5]);
    /// ```
    #[inline]
    pub fn extract_if<F>(&mut self, f: F) -> ExtractIf<T, DropOrder, F, Rebalance>
    where
        F: FnMut(usize, &mut T) -> bool,
    {
        // we are going to start moving stuff to the front so we need to make sure there's space there
        self.reserve_front(1);
        ExtractIf {
            f,
            buffer_write_start: 0,
            devec: self,
            current_index: 0,
            items_removed: 0,
        }
    }

    /// Retains only the elements specified by the predicate function.
    /// In other words, remove all elements `x` such that `f(&mut x)` returns `false`.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec : DeVec<i32> = DeVec::from([1, 2, 3, 4, 5]);
    /// devec.retain_mut(|elem| *elem % 2 == 0);
    /// assert_eq!(devec.as_slice(), &[2, 4]);
    /// ```
    #[inline]
    pub fn retain_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        // use extract_if with a clsoure that ignores the first argument and uses f on the second argument
        let extractor = self.extract_if(|_, elem| !f(elem));
        // drop all elements here
        drop(extractor)
    }

    /// Returns the components of the `DeVec`, consuming the original.
    /// The components are the raw pointer to the buffer, the offset where the DeVec's slice starts, the length, and the capacity of the allocation.
    /// # Safety
    /// The caller is responsible for the use of the raw pointer and the other components.
    ///
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// let (ptr, start, len, cap) = devec.into_raw_parts();
    /// ```
    #[inline]
    #[must_use]
    pub fn into_raw_parts(self) -> (*mut T, usize, usize, usize) {
        let ptr = self.ptr.as_ptr();
        let start = self.start;
        let len = self.len;
        let cap = self.cap;
        std::mem::forget(self);
        (ptr, start, len, cap)
    }

    /// Creates a `DeVec` from the raw components of another `DeVec`.
    /// # Safety
    /// The caller is responsible for ensuring that the raw components are valid.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// let (ptr, start, len, cap) = devec.into_raw_parts();
    /// let devec : DeVec<i32> = unsafe { DeVec::from_raw_parts(ptr, start, len, cap) };
    /// ```
    #[inline]
    #[must_use]
    pub unsafe fn from_raw_parts(ptr: *mut T, start: usize, len: usize, cap: usize) -> Self {
        DeVec {
            ptr: NonNull::new(ptr).unwrap(),
            start,
            len,
            cap,
            drop_order: Default::default(),
            rebalance: Default::default(),
        }
    }

    /// Copies all elements from a slice into the `DeVec`.
    /// # Panics
    /// Panics if the allocation size exceeds [`isize::MAX`]
    /// Other failure happens if an allocation error occurs.
    ///
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.copy_from_slice(&[1, 2, 3]);
    /// assert_eq!(devec.as_slice(), &[1, 2, 3]);
    /// ```
    #[inline]
    pub fn copy_from_slice(&mut self, other: &[T])
    where
        T: Copy,
    {
        self.reserve_back(other.len());
        unsafe {
            // SAFETY: the DeVec has reserved enough space for the slice.
            // SAFETY: We have a &mut to the DeVec, so we know that `other` is not aliasing the inside of this DeVec.
            std::ptr::copy_nonoverlapping(
                other.as_ptr(),
                self.ptr.as_ptr().add(self.start + self.len),
                other.len(),
            );
        }
        self.len += other.len();
    }

    /// Extends the `DeVec` with the elements from the slice.
    /// The elements must be cloned to be inserted into the `DeVec`.
    /// # Panics
    /// Panics if the allocation size exceeds [`isize::MAX`]
    /// Other failure happens if an allocation error occurs.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.extend_from_slice(&[1, 2, 3]);
    /// assert_eq!(devec.as_slice(), &[1, 2, 3]);
    /// ```
    #[inline]
    pub fn extend_from_slice(&mut self, other: &[T])
    where
        T: Clone,
    {
        self.reserve_back(other.len());
        for elem in other {
            self.push_back(elem.clone());
        }
    }

    /// Shifts the elements of the DeVec to be centered in the middle of the buffer.
    /// This can address imbalances in the buffer that may have been caused by repeated pushes or removals on one side.
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec : DeVec<i32> = DeVec::with_capacity(10);
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// devec.push_back(3);
    /// devec.push_back(4);
    /// let old_space_back = devec.space_back();
    /// devec.rebalance();
    /// let new_space_back = devec.space_back();
    /// assert!(new_space_back >= old_space_back);
    /// ```
    #[inline]
    pub fn rebalance(&mut self) {
        let buffer_midpoint = self.cap / 2;
        let new_start = buffer_midpoint - (self.len / 2);
        if self.start == new_start {
            return;
        }
        unsafe {
            std::ptr::copy(
                self.ptr.as_ptr().add(self.start),
                self.ptr.as_ptr().add(new_start),
                self.len,
            );
        }
        self.start = new_start;
    }

    /// Shifts the elements of the DeVec to be centered to the location in the buffer specified by the index.
    /// This can address imbalances in the buffer that may have been caused by repeated pushes or removals on one side.
    ///
    /// # Panics
    /// Panics if the index is larger than the length of the DeVec, or if the index would require writing out of bounds.
    ///
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec : DeVec<i32> = DeVec::with_capacity(10);
    /// devec.push_front(1);
    /// devec.push_front(2);
    /// devec.push_front(3);
    /// devec.push_front(4);
    /// // we should now be heavily imbalanced towards the front
    /// let old_space_front = devec.space_front();
    /// println!("old_space_front: {}", old_space_front);
    /// devec.rebalance_to(5);
    /// // we should be more balanced now
    /// let new_space_front = devec.space_front();
    /// println!("new_space_front: {}", new_space_front);
    /// assert!(new_space_front >= old_space_front);
    /// ```
    #[inline]
    pub fn rebalance_to(&mut self, index: usize) {
        if self.start == index {
            return;
        }
        assert!(
            index < self.cap,
            "attempt to rebalance DeVec of cap={}, with index {} is out of bounds",
            self.cap,
            index
        );
        assert!(
            (self.cap - index) >= self.len,
            "attempt to rebalance DeVec of len={}, cap={} with index {} would write out of bounds",
            self.len,
            self.cap,
            index
        );
        unsafe {
            std::ptr::copy(
                self.ptr.as_ptr().add(self.start),
                self.ptr.as_ptr().add(index),
                self.len,
            );
        }
        self.start = index;
    }

    /// Returns a mutable reference to the DeVec's managed slice. The DeVec is consumed and forgotten, meaning its Drop will never run.
    ///
    /// # Examples
    /// ```
    /// # use spaghetto::DeVec;
    /// let mut devec = DeVec::new();
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// let slice = devec.leak();
    /// assert_eq!(slice, &[1, 2]);
    /// ```
    #[inline]
    #[must_use]
    pub fn leak<'a>(self) -> &'a mut [T] {
        unsafe {
            let slice = std::slice::from_raw_parts_mut(self.ptr.as_ptr().add(self.start), self.len);
            std::mem::forget(self);
            slice
        }
    }

    /// Returns slices of the remaining capacity on each side of the buffer.
    /// The first element of the tuple is the remaining capacity on the front side of the buffer.
    /// The second element of the tuple is the remaining capacity on the back side of the buffer.
    /// # Examples
    /// ```
    /// use spaghetto::DeVec;
    /// let mut devec : DeVec<i32> = DeVec::with_capacity(10);
    /// devec.push_back(1);
    /// devec.push_back(2);
    /// devec.rebalance();
    /// let (front, back) = devec.spare_capacity_mut();
    /// assert!(front.len() >= 4);
    /// assert!(back.len() >= 4);
    /// ```
    #[inline]
    #[must_use]
    pub fn spare_capacity_mut(
        &mut self,
    ) -> (
        &mut [std::mem::MaybeUninit<T>],
        &mut [std::mem::MaybeUninit<T>],
    ) {
        let start = self.start;
        let end = self.start + self.len;
        let spare_front = unsafe {
            std::slice::from_raw_parts_mut(
                self.ptr.as_ptr() as *mut std::mem::MaybeUninit<T>,
                start,
            )
        };
        let spare_back = unsafe {
            std::slice::from_raw_parts_mut(
                self.ptr.as_ptr().add(end) as *mut std::mem::MaybeUninit<T>,
                self.cap - end,
            )
        };
        (spare_front, spare_back)
    }
}

impl<T, DropOrder: DropBehavior, Rebalance> Default for DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn default() -> Self {
        let cap = if std::mem::size_of::<T>() == 0 {
            usize::MAX
        } else {
            0
        };
        DeVec {
            ptr: NonNull::dangling(),
            start: 0,
            len: 0,
            cap,
            drop_order: DropOrder::default(),
            rebalance: Rebalance::default(),
        }
    }
}

/// An iterator over the elements owned by a [`DeVec`]. It pops them from the appropriate side as it goes, and it will drop all remaining elements of the `DeVec` when it is droppped.
pub struct IntoIter<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    devec: DeVec<T, DropOrder, Rebalance>,
}

impl<T, DropOrder, Rebalance> IntoIterator for DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    type Item = T;
    type IntoIter = IntoIter<T, DropOrder, Rebalance>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter { devec: self }
    }
}

impl<'a, T, DropOrder, Rebalance> IntoIterator for &'a DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'a, T, DropOrder, Rebalance> IntoIterator for &'a mut DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}

impl<T, DropOrder, Rebalance> Iterator for IntoIter<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.devec.is_empty() {
            None
        } else {
            self.devec.pop_front()
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.devec.len(), Some(self.devec.len()))
    }
}

impl<T, DropOrder, Rebalance> std::iter::FusedIterator for IntoIter<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
}

impl<T, DropOrder, Rebalance> ExactSizeIterator for IntoIter<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn len(&self) -> usize {
        self.devec.len()
    }
}

impl<T, DropOrder, Rebalance> DoubleEndedIterator for IntoIter<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.devec.pop_back()
    }
}

impl<T, DropOrder, Rebalance> Drop for DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn drop(&mut self) {
        if std::mem::size_of::<T>() == 0 {
            // there's nothing to drop because we do not allocate if T is a ZST
            return;
        }
        if self.cap != 0 {
            while (if DropOrder::IS_INVERTED {
                self.pop_back()
            } else {
                self.pop_front()
            })
            .is_some()
            {}
            let layout = Layout::array::<T>(self.cap).unwrap();
            unsafe {
                std::alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
            }
        }
    }
}

impl<T, DropOrder, Rebalance> Clone for DeVec<T, DropOrder, Rebalance>
where
    T: Clone,
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn clone(&self) -> Self {
        let mut new: DeVec<T, DropOrder, Rebalance> = DeVec::with_capacity(self.cap);
        new.start = (new.cap / 2).saturating_sub(new.len / 2);
        new.len = 0;
        for (i, elem) in self.iter().enumerate() {
            let elem = elem.clone();
            unsafe {
                new.ptr.as_ptr().add(new.start + i).write(elem);
            }
        }
        // We can't trust that elem.clone() won't panic, so we need a strategy for it.
        // Currently, everything is leaked.
        // If we wanted to be able to drop all elements in the event of a panic,
        // we would need to slowly increment new.len after each write
        // so that we have the right len when Drop is run in unwind.
        new.len = self.len;
        new
    }
}

impl<T, DropOrder, Rebalance> Deref for DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    type Target = [T];
    #[inline]
    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr().add(self.start), self.len) }
    }
}

impl<T, DropOrder, Rebalance> DerefMut for DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr().add(self.start), self.len) }
    }
}

impl<T, DropOrder: DropBehavior, Rebalance, R> std::ops::Index<R> for DeVec<T, DropOrder, Rebalance>
where
    R: std::slice::SliceIndex<[T]>,
    Rebalance: RebalanceBehavior,
{
    type Output = <[T] as std::ops::Index<R>>::Output;
    #[inline]
    fn index(&self, index: R) -> &Self::Output {
        self.as_slice().index(index)
    }
}

impl<T, DropOrder, Rebalance> AsRef<[T]> for DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T, DropOrder, Rebalance> AsMut<[T]> for DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T, DropOrder, Rebalance> std::borrow::Borrow<[T]> for DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn borrow(&self) -> &[T] {
        self
    }
}

impl<T, DropOrder, Rebalance> std::borrow::BorrowMut<[T]> for DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn borrow_mut(&mut self) -> &mut [T] {
        self
    }
}

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
#[doc(hidden)]
pub(crate) mod serde_impls {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    impl<T, DropOrder, Rebalance> Serialize for DeVec<T, DropOrder, Rebalance>
    where
        T: Serialize,
        DropOrder: DropBehavior,
        Rebalance: RebalanceBehavior,
    {
        #[inline]
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.as_slice().serialize(serializer)
        }
    }

    impl<'src, T, DropOrder, Rebalance> Deserialize<'src> for DeVec<T, DropOrder, Rebalance>
    where
        T: Deserialize<'src>,
        DropOrder: DropBehavior,
        Rebalance: RebalanceBehavior,
    {
        #[inline]
        fn deserialize<D: Deserializer<'src>>(deserializer: D) -> Result<Self, D::Error> {
            let vec = <Vec<T> as Deserialize<'src>>::deserialize(deserializer)?;
            Ok(DeVec::from(vec).with_drop_order().with_rebalance_behavior())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde_json;
        #[test]
        fn test_serde() {
            let input_sequences = [vec![0, 1, 2, 3, 4, 5, 6], vec![3, 2, 1], vec![]];
            for sequence in input_sequences.iter() {
                let devec = DeVec::from(sequence.clone());
                let serialized = serde_json::to_string(&devec).unwrap();
                let deserialized: DeVec<i32> = serde_json::from_str(&serialized).unwrap();
                assert_eq!(devec, deserialized);
            }
        }
    }
}
/// A draining iterator over the elements of a [`DeVec`].
/// This struct is created by the [`drain`](DeVec::drain) method on [`DeVec`].
pub struct Drain<'a, T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    pub(crate) devec: &'a mut DeVec<T, DropOrder, Rebalance>,
    pub(crate) start: usize,
    pub(crate) current: usize,
    pub(crate) target: usize,
}

impl<'a, T, DropOrder, Rebalance> Iterator for Drain<'a, T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.target {
            None
        } else {
            let val = unsafe { Some(std::ptr::read(self.devec.ptr.as_ptr().add(self.current))) };
            self.current += 1;
            val
        }
    }
}

impl<'a, T, DropOrder, Rebalance> Drop for Drain<'a, T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn drop(&mut self) {
        // TODO: should I bother changing the drop order? for now I always drop from the front
        // drop all remaining elements
        for _ in self.by_ref() {}

        // don't look at pointers past the end of the filled buffer space
        if self.devec.len + self.devec.start <= self.target {
            self.devec.len -= self.target - self.start;
            return;
        }

        // shift the the items after the drained items to the left
        let start_of_remaining_elements = self.target;
        let where_they_need_to_go = self.start;
        let num_items_after_end = (self.devec.start + self.devec.len) - self.target;
        // move the data
        unsafe {
            std::ptr::copy(
                self.devec.ptr.as_ptr().add(start_of_remaining_elements),
                self.devec.ptr.as_ptr().add(where_they_need_to_go),
                num_items_after_end,
            );
        }
        self.devec.len -= self.target - self.start;
    }
}

/// An iterator that removes elements from the [`DeVec`] that satisfy a predicate.
/// This struct is created by the [`extract_if`](DeVec::extract_if) method on [`DeVec`].
pub struct ExtractIf<'a, T, D, F, R>
where
    D: DropBehavior,
    F: FnMut(usize, &mut T) -> bool,
    R: RebalanceBehavior,
{
    f: F,
    devec: &'a mut DeVec<T, D, R>,
    current_index: usize,
    buffer_write_start: usize,
    items_removed: usize,
}

impl<'a, T, D, F, R> Iterator for ExtractIf<'a, T, D, F, R>
where
    D: DropBehavior,
    F: FnMut(usize, &mut T) -> bool,
    R: RebalanceBehavior,
{
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.current_index < self.devec.len {
            let current_index = self.current_index;
            let current_ptr = unsafe {
                self.devec
                    .ptr
                    .as_ptr()
                    .add(self.devec.start + current_index)
            };
            let mut result = unsafe { std::ptr::read(current_ptr) };
            #[cfg(test)]
            {
                println!(
                    "starting iteration with current index {current_index} and buffer write start {}",
                    self.buffer_write_start
                );
            }

            let should_extract = (self.f)(current_index, &mut result);
            if should_extract {
                #[cfg(test)]
                {
                    println!("extracting");
                }
                self.current_index += 1;
                self.items_removed += 1;
                return Some(result);
            } else {
                #[cfg(test)]
                {
                    println!(
                        "moving element {} to position {}",
                        self.current_index, self.buffer_write_start
                    );
                }
                // copy the element to the front of the buffer
                let front_of_buffer_ptr = unsafe {
                    self.devec
                        .ptr
                        .as_ptr()
                        .add(self.devec.start + self.buffer_write_start)
                };
                unsafe {
                    std::ptr::write(front_of_buffer_ptr, result);
                }
                self.buffer_write_start += 1;
                self.current_index += 1;
            }
        }
        let old_len = self.devec.len;
        let new_len = old_len - self.items_removed;
        self.devec.len = new_len;
        // this prevents us from accidentally truncating the buffer if next is called again after we've already finished
        self.items_removed = 0;
        None
    }
}

impl<'a, T, D, F, R> Drop for ExtractIf<'a, T, D, F, R>
where
    D: DropBehavior,
    R: RebalanceBehavior,
    F: FnMut(usize, &mut T) -> bool,
{
    #[inline]
    fn drop(&mut self) {
        // drop all remaining elements
        for _ in self.by_ref() {}
    }
}

impl<T, DropOrder, Rebalance> FromIterator<T> for DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        DeVec::from(iter.into_iter().collect::<Vec<T>>())
            .with_drop_order()
            .with_rebalance_behavior()
    }
}

impl<T, DropOrder, Rebalance> Extend<T> for DeVec<T, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.push_back(item);
        }
    }
}

impl<T> From<Vec<T>> for DeVec<T> {
    #[inline]
    fn from(vec: Vec<T>) -> Self {
        let (ptr, len, cap) = (vec.as_ptr(), vec.len(), vec.capacity());
        std::mem::forget(vec);
        DeVec {
            ptr: NonNull::new(ptr as *mut T).unwrap(),
            start: 0,
            len,
            cap,
            drop_order: Default::default(),
            rebalance: Default::default(),
        }
    }
}

impl<T, const N: usize> From<[T; N]> for DeVec<T> {
    #[inline]
    fn from(array: [T; N]) -> Self {
        From::from(Vec::from(array))
    }
}

#[cfg(test)]
mod devec_test_from {
    use super::*;
    #[test]
    fn test_from_vec() {
        let vec = vec![1, 2, 3, 4, 5];
        let devec: DeVec<i32> = DeVec::from(vec);
        assert_eq!(devec.len(), 5);
        assert_eq!(devec.capacity(), 5);
        assert_eq!(devec[0], 1);
        assert_eq!(devec[1], 2);
        assert_eq!(devec[2], 3);
        assert_eq!(devec[3], 4);
        assert_eq!(devec[4], 5);
    }

    #[test]
    fn test_from_vec2() {
        let vec = vec![2, 3, 5];
        let mut devec: DeVec<i32> = DeVec::from(vec);
        assert_eq!(devec.len(), 3);
        assert_eq!((&*devec), &[2, 3, 5][..]);

        devec.pop_back();
        devec.push_front(1);
        devec.push_back(6);
        assert_eq!(devec.len(), 4);
        assert_eq!(&devec, &[1, 2, 3, 6][..]);
    }
}

// partialeq, eq, partialord, ord, and hash implementations
impl<T, DropOrder, Rebalance> PartialEq for DeVec<T, DropOrder, Rebalance>
where
    T: PartialEq,
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}

impl<T, DropOrder, Rebalance> PartialEq<[T]> for DeVec<T, DropOrder, Rebalance>
where
    T: PartialEq,
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn eq(&self, other: &[T]) -> bool {
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}

impl<T, DropOrder, Rebalance> Eq for DeVec<T, DropOrder, Rebalance>
where
    T: Eq,
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
}

impl<T, DropOrder, Rebalance> PartialOrd for DeVec<T, DropOrder, Rebalance>
where
    T: PartialOrd,
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}

impl<T, DropOrder, Rebalance> PartialOrd<[T]> for DeVec<T, DropOrder, Rebalance>
where
    T: PartialOrd,
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn partial_cmp(&self, other: &[T]) -> Option<std::cmp::Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}

impl<T, DropOrder, Rebalance> Ord for DeVec<T, DropOrder, Rebalance>
where
    T: Ord,
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.iter().cmp(other.iter())
    }
}

impl<T, DropOrder, Rebalance> std::hash::Hash for DeVec<T, DropOrder, Rebalance>
where
    T: std::hash::Hash,
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl<DropOrder, Rebalance> std::io::Write for DeVec<u8, DropOrder, Rebalance>
where
    DropOrder: DropBehavior,
    Rebalance: RebalanceBehavior,
{
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.copy_from_slice(buf);
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A version of the [`vec!`] macro but for [`DeVec`]. This macro helps you create a [`DeVec`] from a list of elements.
/// It will use the default drop order of [`FrontToBack`](crate::settings::FrontToBack) and rebalance strategy of putting the center of the elements into the [`Middle`](crate::settings::Middle) of the allocation after a resize.
/// # Examples
/// ```
/// # use spaghetto::devec;
/// let devec = devec![1, 2, 3, 4, 5];
/// assert_eq!(devec.len(), 5);
/// assert_eq!(devec.capacity(), 5);
/// assert_eq!(devec.as_slice(), &[1, 2, 3, 4, 5]);
/// ```
/// ```
/// # use spaghetto::devec;
/// let devec = devec![1; 5];
/// assert_eq!(devec.len(), 5);
/// assert_eq!(devec.capacity(), 5);
/// assert_eq!(devec.as_slice(), &[1, 1, 1, 1, 1]);
/// ```
/// ```
/// # use spaghetto::{DeVec, devec};
/// let devec : DeVec<i32> = devec![];
/// assert_eq!(devec.len(), 0);
/// assert_eq!(devec.capacity(), 0);
/// ```
#[macro_export]
macro_rules! devec {
    () => {
        $crate::devec::DeVec::<_>::new()
    };
    ($elem:expr; $n:expr) => {
        $crate::devec::DeVec::<_>::from_iter(std::iter::repeat($elem).take($n))
    };
    ($($x:expr),+ $(,)?) => {
        $crate::devec::DeVec::<_>::from(vec![$($x),+])
    };
}

#[cfg(test)]
mod devec_tests {
    use super::*;

    #[test]
    pub fn push_back_order() {
        let mut devec: DeVec<i32> = DeVec::new();
        devec.push_back(1);
        devec.push_back(2);
        devec.push_back(3);
        assert_eq!(&*devec, &[1, 2, 3]);
    }
    #[test]
    pub fn pop_back_order() {
        let mut devec = DeVec::new();
        devec.push_back(1);
        devec.push_back(2);
        devec.push_back(3);
        assert_eq!(devec.pop_back(), Some(3));
        assert_eq!(devec.pop_back(), Some(2));
        assert_eq!(devec.pop_back(), Some(1));
        assert_eq!(devec.pop_back(), None);
    }

    #[test]
    pub fn push_front_order() {
        let mut devec = DeVec::new();
        devec.push_front(1);
        devec.push_front(2);
        devec.push_front(3);
        assert_eq!(&*devec, &[3, 2, 1]);
    }
    #[test]
    pub fn pop_front_order() {
        let mut devec = DeVec::new();
        devec.push_front(1i32);
        devec.push_front(2);
        devec.push_front(3);
        assert_eq!(devec.pop_front(), Some(3));
        assert_eq!(devec.pop_front(), Some(2));
        assert_eq!(devec.pop_front(), Some(1));
        assert_eq!(devec.pop_front(), None);
    }
    #[test]
    pub fn test_interleave_push_order() {
        let mut devec = DeVec::new();
        devec.push_front(1i32);
        devec.push_back(2);
        devec.push_front(3);
        devec.push_back(4);
        devec.push_front(5);
        devec.push_back(6);
        assert_eq!(&*devec, &[5, 3, 1, 2, 4, 6]);
    }

    #[test]
    pub fn test_interleave_pop_order() {
        let mut devec = DeVec::new();
        devec.push_front(1i32);
        devec.push_back(2);
        devec.push_front(3);
        devec.push_back(4);
        devec.push_front(5);
        devec.push_back(6);
        assert_eq!(devec.pop_front(), Some(5));
        assert_eq!(devec.pop_back(), Some(6));
        assert_eq!(devec.pop_front(), Some(3));
        assert_eq!(devec.pop_front(), Some(1));
        assert_eq!(devec.pop_back(), Some(4));
        assert_eq!(devec.pop_back(), Some(2));
        assert_eq!(devec.pop_front(), None);
        assert_eq!(devec.pop_back(), None);
    }

    #[test]
    pub fn test_zst_operations() {
        let mut devec = DeVec::<()>::new();
        devec.push_back(());
        devec.push_front(());
        devec.push_back(());
        devec.push_front(());
        devec.push_back(());
        devec.push_back(());
        devec.push_front(());
        devec.push_back(());
        devec.push_front(());
        let target_len = 9;
        assert_eq!(devec.len(), target_len);
        assert_eq!(devec.pop_front(), Some(()));
        assert_eq!(devec.len(), target_len - 1);
    }

    #[test]
    pub fn test_collect() {
        let devec: DeVec<i32> = (0..10).collect();
        assert_eq!(&*devec, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    // extract test 1: extract if an element is odd
    #[test]
    pub fn test_extract_if() {
        let mut devec: DeVec<_> = DeVec::from_iter(0..10);
        let extracted: Vec<i32> = devec.extract_if(|_, x| *x % 2 == 1).collect();
        assert_eq!(extracted, vec![1, 3, 5, 7, 9]);
        assert_eq!(&*devec, &[0, 2, 4, 6, 8]);
    }

    // extract test 2: extract if an element is the first one
    #[test]
    pub fn test_extract_if2() {
        let mut devec: DeVec<_> = DeVec::from_iter(0..10);
        let mut is_first = true;
        let extracted: Vec<i32> = devec
            .extract_if(|_, _x| {
                if is_first {
                    is_first = false;
                    true
                } else {
                    false
                }
            })
            .collect();
        assert_eq!(extracted, vec![0]);
        assert_eq!(&*devec, &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }
    // extract test 2: extract if an element's index is in a certain range
    #[test]
    pub fn test_extract_if3() {
        let mut devec: DeVec<_> = DeVec::from_iter(0..10);
        let index_range = 0..3;
        let extracted: Vec<i32> = devec.extract_if(|i, _| index_range.contains(&i)).collect();
        assert_eq!(&*extracted, &[0, 1, 2]);
        assert_eq!(&*devec, &[3, 4, 5, 6, 7, 8, 9]);

        let mut devec: DeVec<_> = DeVec::from_iter(0..10);
        let keep_everything_before = 6;
        let extracted: Vec<i32> = devec
            .extract_if(|i, _| i >= keep_everything_before)
            .collect();
        assert_eq!(&*extracted, &[6, 7, 8, 9]);
        assert_eq!(&*devec, &[0, 1, 2, 3, 4, 5]);

        let empty_i32_slice: &[i32] = &[];
        let mut devec: DeVec<_> = DeVec::from_iter(0..10);
        let keep_everything_before = 42;
        let extracted: Vec<i32> = devec
            .extract_if(|i, _| i >= keep_everything_before)
            .collect();
        assert_eq!(&*extracted, empty_i32_slice);
        assert_eq!(&*devec, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

        let mut devec: DeVec<_> = DeVec::from_iter(0..10);
        let keep_everything_before = 0;
        let extracted: Vec<i32> = devec
            .extract_if(|i, _| i >= keep_everything_before)
            .collect();
        assert_eq!(&*extracted, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(&*devec, empty_i32_slice);
    }
}
