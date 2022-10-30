#![no_std]

use core::{
    mem::MaybeUninit,
    ops::{Index, IndexMut},
};

pub struct Deque<T, const CAPACITY: usize> {
    /// The index of the first element stored in `data`, if non-empty.
    start: usize,

    /// The first index past the last element stored in `data`.
    end: usize,

    /// The number of elements stored in `data`.
    ///
    /// Always congruent with `end - start` modulo `CAPACITY`, in other
    len: usize,

    /// A circular buffer.
    ///
    /// Data is stored in a single contiguous region that may wrap around the
    /// array boundary (which technically causes a discontinuity creating two
    /// separate contiguous regions in the linear memory space, but generally
    /// you can conceptually think of it as a single region).
    ///
    /// Elements are stored at consecutive indexes after and including `start`
    /// and at indexes before `end`.
    ///
    /// - If `start < end`, they are stored in-order in the range of indexes
    /// `start..end`.
    ///
    /// - If `start > end`, they are stored in two sub-arrays with index ranges
    /// `start..CAPACITY` and `0..end`.
    ///
    /// - If `start == end`, it is an ambiguous case, the buffer may either be
    /// full or empty, and the `len` field should be used to disambiguate this
    /// case.
    ///
    /// In all cases, the `len` field tracks the total number of elements in
    /// all valid regions:
    ///
    /// - If `start < end`, then `len` is equal to `end - start`.
    /// - If `start > end`, then `len` is equal to `CAPACITY + end - start`.
    /// - If `start == end`, then `len` is either equal to `0` or `CAPACITY`.
    data: [MaybeUninit<T>; CAPACITY],
}

impl<T, const CAPACITY: usize> Deque<T, CAPACITY> {
    /// Creates an empty deque.
    ///
    /// # Examples
    ///
    /// ```
    /// use fullhouse::Deque;
    ///
    /// let deque: Deque<u32, 8> = Deque::new();
    /// ```
    pub const fn new() -> Self {
        Self {
            // Empty state:
            start: 0,
            end: 0,
            len: 0,

            // Safety: The value inside the outermost MaybeUninit must be valid.
            // - A value of `[MaybeUninit<T>; N]` is valid because a value of
            //  `MaybeUninit<T>` is always valid (even if the inner value
            //  isn't).
            //
            // This is the same as the unstable `MaybeUninit::uninit_array()` at
            // the time of writing.
            data: unsafe { MaybeUninit::<[MaybeUninit<T>; CAPACITY]>::uninit().assume_init() },
        }
    }

    /// The maximum number of elements this deque can hold.
    ///
    /// # Examples
    ///
    /// ```
    /// use fullhouse::Deque;
    ///
    /// let deque: Deque<u32, 10> = Deque::new();
    /// assert_eq!(deque.capacity(), 10);
    /// ```
    pub const fn capacity(&self) -> usize {
        CAPACITY
    }

    /// The number of elements currently in the deque.
    ///
    /// # Examples
    ///
    /// ```
    /// use fullhouse::Deque;
    ///
    /// let mut deque: Deque<i32, 8> = Deque::new();
    /// assert_eq!(deque.len(), 0);
    /// deque.push_back(1);
    /// assert_eq!(deque.len(), 1);
    /// ```
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the deque is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut deque = VecDeque::new();
    /// assert!(deque.is_empty());
    /// deque.push_front(1);
    /// assert!(!deque.is_empty());
    /// ```
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns `true` if the deque is full.
    ///
    /// # Examples
    ///
    /// ```
    /// use fullhouse::Deque;
    ///
    /// let mut deque: Deque<i32, 4> = Deque::new();
    /// deque.push_front(1);
    /// assert!(!deque.is_full());
    /// deque.push_front(2);
    /// assert!(!deque.is_full());
    /// deque.push_front(3);
    /// assert!(!deque.is_full());
    /// deque.push_front(4);
    /// assert!(deque.is_full());
    /// ```
    pub const fn is_full(&self) -> bool {
        self.len == CAPACITY
    }

    /// Clears the deque, removing all values.
    ///
    /// # Examples
    ///
    /// ```
    /// use fullhouse::Deque;
    ///
    /// let mut deque: Deque<i32, 4> = Deque::new();
    /// deque.push_back(1);
    /// deque.clear();
    /// assert!(deque.is_empty());
    /// ```
    pub fn clear(&mut self) {
        // Get a list of valid indexes.
        let indexes = self.indexes();

        // Reset region state:
        self.start = 0;
        self.end = 0;
        self.len = 0;

        // Drop all known-valid values in the array:
        for idx in indexes {
            // Safety: The value in the MaybeUninit must be valid.
            // - indexes() yields an iterator of indexes that are all valid by
            //   definition (as defined in the docstring for `data`).
            // - indexes() yields each index only once, so the value has not
            //   been invalidated by an earlier loop iteration.
            //
            // Postcondition: The value in the MaybeUninit is invalidated.
            // - The iterator yields each index only once - it will not be
            //   accessed again in this loop.
            // - The region is reset to empty immediately after the iterator is
            //   constructed and before this loop executes, so later code
            //   (including panics) will not assume that this data is valid.
            unsafe { self.data[idx].assume_init_drop() };
        }
    }

    /// Prepends an element to the deque.
    ///
    /// # Examples
    ///
    /// ```
    /// use fullhouse::Deque;
    ///
    /// let mut d: Deque<i32, 4> = Deque::new();
    /// d.push_front(1);
    /// d.push_front(2);
    /// assert_eq!(d.pop_front(), Some(2));
    /// ```
    pub fn push_front(&mut self, value: T) -> Result<(), T> {
        if self.is_full() {
            Err(value)
        } else {
            // Insert value before the beginning of the region:
            let new_start = (self.start + CAPACITY - 1) % CAPACITY;
            self.data[new_start].write(value);

            // Expand region to include new element:
            self.start = new_start;
            self.len += 1;
            Ok(())
        }
    }

    /// Appends an element to the back of the deque.
    ///
    /// # Examples
    ///
    /// ```
    /// use fullhouse::Deque;
    ///
    /// let mut buf: Deque<i32, 4> = Deque::new();
    /// buf.push_back(1);
    /// buf.push_back(3);
    /// assert_eq!(buf.pop_back(), Some(3));
    /// ```
    pub fn push_back(&mut self, value: T) -> Result<(), T> {
        if self.is_full() {
            Err(value)
        } else {
            // Insert the value after the end of the region:
            self.data[self.end].write(value);

            // Expand region to include new element:
            self.end = (self.end + 1) % CAPACITY;
            self.len += 1;
            Ok(())
        }
    }

    /// Removes the first element and returns it, or `None` if the deque is
    /// empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use fullhouse::Deque;
    ///
    /// let mut d: Deque<i32, 4> = Deque::new();
    /// d.push_back(1);
    /// d.push_back(2);
    ///
    /// assert_eq!(d.pop_front(), Some(1));
    /// assert_eq!(d.pop_front(), Some(2));
    /// assert_eq!(d.pop_front(), None);
    /// ```
    pub fn pop_front(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            // Shrink region by 1 element from start.
            let old_start = self.start;
            self.start = (old_start + 1) % CAPACITY;
            self.len -= 1;

            // Safety: The value in the MaybeUninit must be valid.
            // - At the start of the function, it was in the valid region of the
            //   `data` array, and is not otherwise accessed in this function.
            //
            // Postcondition: The value in the MaybeUninit is invalidated (it
            // has been moved).
            // - The region has already been shrunk, so later code (including
            //   panics) will not assume that this index is valid.
            let value = unsafe { self.data[old_start].assume_init_read() };
            Some(value)
        }
    }

    /// Removes the last element from the deque and returns it, or `None` if
    /// it is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use fullhouse::Deque;
    ///
    /// let mut buf: Deque<i32, 4> = Deque::new();
    /// assert_eq!(buf.pop_back(), None);
    /// buf.push_back(1);
    /// buf.push_back(3);
    /// assert_eq!(buf.pop_back(), Some(3));
    /// ```
    pub fn pop_back(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            // Shrink region by 1 element from end.
            self.end = (self.end + CAPACITY - 1) % CAPACITY;
            self.len -= 1;

            // Safety: The value in the MaybeUninit must be valid.
            // - At the start of the function, it was in the valid region of the
            //   `data` array, and is not otherwise accessed in this function.
            //
            // Postcondition: The value in the MaybeUninit is invalidated (it
            // has been moved)
            // - The region has already been shrunk, so later code (including
            //   panics) will not assume that this index is valid.
            let value = unsafe { self.data[self.end].assume_init_read() };
            Some(value)
        }
    }

    /// Provides a reference to the element at the given index.
    ///
    /// Element at index 0 is at the front of the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use fullhouse::Deque;
    ///
    /// let mut buf: Deque<i32, 4> = Deque::new();
    /// buf.push_back(3);
    /// buf.push_back(4);
    /// buf.push_back(5);
    /// assert_eq!(buf.get(1), Some(&4));
    /// ```
    pub fn get(&self, index: usize) -> Option<&T> {
        self.data_index(index).map(|idx| {
            // Safety: The value in the MaybeUninit must be valid.
            // This is guaranteed by `data_index`, which will only return
            // `Some` if the index points to a valid, initialized element.
            unsafe { self.data[idx].assume_init_ref() }
        })
    }

    /// Provides a mutable reference to the element at the given index.
    ///
    /// Element at index 0 is at the front of the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use fullhouse::Deque;
    ///
    /// let mut buf: Deque<i32, 4> = Deque::new();
    /// buf.push_back(3);
    /// buf.push_back(4);
    /// buf.push_back(5);
    /// if let Some(elem) = buf.get_mut(1) {
    ///     *elem = 7;
    /// }
    ///
    /// assert_eq!(buf[1], 7);
    /// ```
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.data_index(index).map(|idx| {
            // Safety: The value in the MaybeUninit must be valid.
            // This is guaranteed by `data_index`, which will only return
            // `Some` if the index points to a valid, initialized element.
            unsafe { self.data[idx].assume_init_mut() }
        })
    }

    /// Provides a reference to the front element, or `None` if the deque is
    /// empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use fullhouse::Deque;
    ///
    /// let mut d: Deque<i32, 4> = Deque::new();
    /// assert_eq!(d.front(), None);
    ///
    /// d.push_back(1);
    /// d.push_back(2);
    /// assert_eq!(d.front(), Some(&1));
    /// ```
    pub fn front(&self) -> Option<&T> {
        self.get(0)
    }

    /// Provides a mutable reference to the front element, or `None` if the
    /// deque is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use fullhouse::Deque;
    ///
    /// let mut d: Deque<i32, 4> = Deque::new();
    /// assert_eq!(d.front_mut(), None);
    ///
    /// d.push_back(1);
    /// d.push_back(2);
    /// match d.front_mut() {
    ///     Some(x) => *x = 9,
    ///     None => (),
    /// }
    /// assert_eq!(d.front(), Some(&9));
    /// ```
    pub fn front_mut(&mut self) -> Option<&mut T> {
        self.get_mut(0)
    }

    /// Provides a reference to the back element, or `None` if the deque is
    /// empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use fullhouse::Deque;
    ///
    /// let mut d: Deque<i32, 4> = Deque::new();
    /// assert_eq!(d.back(), None);
    ///
    /// d.push_back(1);
    /// d.push_back(2);
    /// assert_eq!(d.back(), Some(&2));
    /// ```
    pub fn back(&self) -> Option<&T> {
        self.get(self.len().wrapping_sub(1))
    }

    /// Provides a mutable reference to the back element, or `None` if the
    /// deque is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use fullhouse::Deque;
    ///
    /// let mut d: Deque<i32, 4> = Deque::new();
    /// assert_eq!(d.back(), None);
    ///
    /// d.push_back(1);
    /// d.push_back(2);
    /// match d.back_mut() {
    ///     Some(x) => *x = 9,
    ///     None => (),
    /// }
    /// assert_eq!(d.back(), Some(&9));
    /// ```
    pub fn back_mut(&mut self) -> Option<&mut T> {
        self.get_mut(self.len().wrapping_sub(1))
    }

    /// Indexes of valid values in the data array, in logical order from `start`
    /// to `end`.
    fn indexes(&self) -> impl Iterator<Item = usize> {
        // A bit of a workaround for the type system - some of these branches
        // could produce a simpler type than `Chain<Range, Range>`
        // but the function _must_ have a single return type. So, all branches
        // create two ranges, and create additional empty ranges if needed.
        let (first, second) = if self.is_empty() {
            (0..0, 0..0)
        } else if self.start < self.end {
            (self.start..self.end, 0..0)
        } else {
            (self.start..CAPACITY, 0..self.end)
        };
        first.chain(second)
    }

    /// Compute an index into the `data` array given the offset from `start`.
    ///
    /// This is guaranteed to return an index to a valid element. If the index
    /// would point outside the valid range(s) (i.e., if `offset >= len`), this
    /// function will instead return `None`.
    fn data_index(&self, offset: usize) -> Option<usize> {
        if offset < self.len() {
            // Check whether index wraps around the end of `data`.
            //
            // This check basically lets us implement `(self.start + offset) %
            // CAPACITY` without causing any wrapping arithmetic or using
            // modulo.
            //
            // I don't _think_ anyone will use this with capacities near the
            // size limit of `usize`, but you never know.
            let pre_wrap_size = CAPACITY - self.start;
            if offset < pre_wrap_size {
                Some(self.start + offset)
            } else {
                Some(offset - pre_wrap_size)
            }
        } else {
            None
        }
    }
}

impl<T, const CAPACITY: usize> Drop for Deque<T, CAPACITY> {
    fn drop(&mut self) {
        // Drops any elements still in the deque:
        self.clear();
    }
}

impl<T, const CAPACITY: usize> Index<usize> for Deque<T, CAPACITY> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("Out of bounds access")
    }
}

impl<T, const CAPACITY: usize> IndexMut<usize> for Deque<T, CAPACITY> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).expect("Out of bounds access")
    }
}
