#![no_std]

use core::mem::MaybeUninit;

pub struct Deque<T, const CAPACITY: usize> {
    // The index of the first element stored in `data`, if non-empty.
    start: usize,

    // The first index past the last element stored in `data`.
    end: usize,

    // The number of elements stored in `data`.
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
    pub fn new() -> Self {
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
    pub fn capacity(&self) -> usize {
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
    pub fn len(&self) -> usize {
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
    pub fn is_empty(&self) -> bool {
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
    pub fn is_full(&self) -> bool {
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

    // Indexes of valid values in the data array, in logical order from `start`
    // to `end`.
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
}

impl<T, const CAPACITY: usize> Drop for Deque<T, CAPACITY> {
    fn drop(&mut self) {
        // Drops any elements still in the deque:
        self.clear();
    }
}
