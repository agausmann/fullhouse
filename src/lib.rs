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

    pub fn capacity(&self) -> usize {
        CAPACITY
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len == CAPACITY
    }

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

    pub fn push_front(&mut self, value: T) -> Result<(), T> {
        if self.is_full() {
            Err(value)
        } else {
            // Insert value before the beginning of the region:
            let new_start = (self.start + CAPACITY - 1) % CAPACITY;
            self.data[self.start].write(value);

            // Expand region to include new element:
            self.start = new_start;
            self.len += 1;
            Ok(())
        }
    }

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

    pub fn pop_front(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            // Shrink region by 1 element from start.
            let old_start = (self.start + 1) % CAPACITY;
            self.start = (self.start + 1) % CAPACITY;
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
