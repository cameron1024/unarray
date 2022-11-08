use core::iter::FromIterator;
use crate::{mark_initialized, uninit_buf};

/// A wrapper type to collect an [`Iterator`] into an array
///
/// ```
/// # use unarray::*;
/// let iter = vec![1, 2, 3].into_iter();
/// let ArrayFromIter(array) = iter.collect();
///
/// assert_eq!(array, Some([1, 2, 3]));
/// ```
/// Since iterators don't carry compile-time information about their length (even
/// [`core::iter::ExactSizeIterator`] only provides this at runtime), collecting may fail if the
/// iterator doesn't yield **exactly** `N` elements:
/// ```
/// use unarray::*;
/// let too_many = vec![1, 2, 3, 4].into_iter();
/// let ArrayFromIter::<i32, 3>(option) = too_many.collect();
/// assert!(option.is_none());
///
/// let too_few = vec![1, 2].into_iter();
/// let ArrayFromIter::<i32, 3>(option) = too_few.collect();
/// assert!(option.is_none());
/// ```
pub struct ArrayFromIter<T, const N: usize>(pub Option<[T; N]>);

impl<T, const N: usize> FromIterator<T> for ArrayFromIter<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut buffer = uninit_buf::<T, N>();
        let mut iter = iter.into_iter();
        let mut buf_iter = buffer.iter_mut();

        loop {
            let item = iter.next();
            let slot = buf_iter.next();

            match (item, slot) {
                (Some(item), Some(slot)) => slot.write(item),
                (Some(_), None) => return Self(None),
                (None, Some(_)) => return Self(None),
                // SAFETY
                // If this is reached, every prior iteration of the loop has matched
                // (Some(_), Some(_)). As such, both iterators have yielded the same number of
                // elements, so every slot has been written to
                (None, None) => return Self(Some(unsafe { mark_initialized(buffer) })),
            };
        }
    }
}

#[cfg(test)]
mod tests {

    use core::convert::TryInto;

    use crate::testing::vec_strategy;
    use proptest::{prop_assert, prop_assert_eq};
    use test_strategy::proptest;

    use super::*;

    #[test]
    fn can_collect_array_from_iter() {
        let iter = vec![1, 2, 3].into_iter();

        let ArrayFromIter(array) = iter.collect();
        assert_eq!(array.unwrap(), [1, 2, 3]);
    }

    #[test]
    fn fails_if_incorrect_number_of_elements() {
        let iter = [1, 2, 3].iter();
        let ArrayFromIter::<_, 4>(array) = iter.collect();
        assert!(array.is_none());

        let iter = [1, 2, 3].iter();
        let ArrayFromIter::<_, 2>(array) = iter.collect();
        assert!(array.is_none());
    }

    const LEN: usize = 100;
    const SHORT_LEN: usize = LEN - 1;
    const LONG_LEN: usize = LEN + 1;

    #[proptest]
    #[cfg_attr(miri, ignore)]
    fn undersized_proptest(#[strategy(vec_strategy(LEN))] vec: Vec<String>) {
        let ArrayFromIter::<String, SHORT_LEN>(array) = vec.into_iter().collect();
        prop_assert!(array.is_none());
    }

    #[proptest]
    #[cfg_attr(miri, ignore)]
    fn oversized_proptest(#[strategy(vec_strategy(LEN))] vec: Vec<String>) {
        let ArrayFromIter::<String, LONG_LEN>(array) = vec.into_iter().collect();
        prop_assert!(array.is_none());
    }

    #[proptest]
    #[cfg_attr(miri, ignore)]
    fn just_right_proptest(#[strategy(vec_strategy(LEN))] vec: Vec<String>) {
        let expected: [String; LEN] = vec.clone().try_into().unwrap();
        let ArrayFromIter(array) = vec.into_iter().collect();
        prop_assert_eq!(array.unwrap(), expected);
    }
}


