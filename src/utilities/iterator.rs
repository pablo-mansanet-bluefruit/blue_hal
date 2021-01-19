pub trait Unique {
    fn all_unique(self) -> bool;
}

impl<T: Clone + Iterator<Item = I>, I: PartialEq> Unique for T {
    fn all_unique(mut self) -> bool {
        // O(n^2), could do with optimisation. Difficult
        // to optimise without a hash set (no heap)
        while let Some(element) = self.next() {
            if self.clone().any(|e| e == element) {
                return false;
            }
        }
        true
    }
}

/// Iterates until a sequence is reached (stops before it)
pub trait UntilSequence<T>: Iterator<Item=T> + Sized {
    fn until_sequence(self, sequence: &[T]) -> UntilSequenceIterator<T, Self>;
}

pub struct UntilSequenceIterator<'a, T, I: Iterator<Item=T>> {
    inner: I,
    sequence: &'a [T],
    head: usize,
    tail: usize,
    divergent: Option<T>, // First value that diverges from the sequence
}

impl<'a, T, I: Iterator<Item=T>> UntilSequence<T> for I {
    fn until_sequence(self, sequence: &[T]) -> UntilSequenceIterator<T, Self> {
        UntilSequenceIterator { inner: self, sequence, head: 0, tail: 0, divergent: None }
    }
}

impl<'a, T: Copy + PartialEq, I: Iterator<Item=T>> Iterator for UntilSequenceIterator<'a, T, I> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        // The sequence has been found while iterating, so we stop here
        if self.tail == self.sequence.len() {
            return None;
        }

        loop {
            if self.divergent.is_some() {
                // A value has diverged from the secuence, so we track
                // the sequence back up until we return the divergent.
                break self.iterate_on_sequence()
            } else if let Some(candidate) = self.inner.next() {
                if candidate == self.sequence[self.tail] {
                    self.tail += 1;
                } else {
                    self.divergent = Some(candidate);
                }
            } else {
                break self.iterate_on_sequence();
            }
        }
    }
}

impl<'a, T: Copy + PartialEq, I: Iterator<Item=T>> UntilSequenceIterator<'a, T, I> {

    // Each call to this function retrieves one element from the sequence until
    // reaching the divergent. It then retrieves the divergent and clears it.
    // If there is no divergent, simply returns the sequence until consumed.
    fn iterate_on_sequence(&mut self) -> Option<T> {
        if self.head == self.tail {
            let divergent = self.divergent.take();
            self.tail = 0;
            self.head = 0;
            divergent
        } else {
            self.head += 1;
            Some(self.sequence[self.head -1])
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn all_unique_in_various_scenarios() {
        assert!([3, 4, 1, 5].iter().all_unique());
        assert!(![1, 2, 3, 3, 2].iter().all_unique());
        assert!(["fish", "foot", "fly", "foresight"].iter().all_unique());
        assert!(![None, Some(3), Some(5), None].iter().all_unique());
    }


    #[test]
    fn iterating_until_sequence() {
        assert!([3, 4, 1, 5].iter().all_unique());
        assert!(![1, 2, 3, 3, 2].iter().all_unique());
        assert!(["fish", "foot", "fly", "foresight"].iter().all_unique());
        assert!(![None, Some(3), Some(5), None].iter().all_unique());
    }
}
