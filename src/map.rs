use crate::{mark_initialized, uninit_buf};

/// An extension trait that adds methods to `[T; N]`
///
/// This trait provides [`UnarrayArrayExt::map_result`] and [`UnarrayArrayExt::map_option`], 
/// which provide functionality similar to the nightly-only [`array::try_map`]
pub trait UnarrayArrayExt<T, const N: usize> {
    /// Maps an array, short-circuiting if any element produces an `Err`
    ///
    /// ```
    /// # use unarray::*;
    /// let elements = ["123", "234", "345"];
    /// let mapped = elements.map_result(|s| s.parse());
    /// assert_eq!(mapped, Ok([123, 234, 345]));
    /// ```
    ///
    /// This function applies `f` to every element. If any element produces an `Err`, the function
    /// immediately returns that error. Otherwise, it returns `Ok(result)` where `result` contains
    /// the mapped elements in an array.
    ///
    /// This function does not allocate space on the heap
    ///
    /// For functions that return an `Option`, consider using [`UnarrayArrayExt::map_option`]
    fn map_result<S, E>(self, f: impl FnMut(T) -> Result<S, E>) -> Result<[S; N], E>;

    /// Maps an array, short-circuiting if any element produces a `None`
    ///
    /// ```
    /// # use unarray::*;
    /// fn parse(s: &str) -> Option<bool> {
    ///   match s {
    ///     "true" => Some(true),
    ///     "false" => Some(false),
    ///     _ => None,
    ///   }
    /// }
    ///
    /// let elements = ["true", "false", "true"];
    /// let mapped = elements.map_option(parse);
    /// assert_eq!(mapped, Some([true, false, true]));
    /// ```
    ///
    /// This function applies `f` to every element. If any element produces `None`, the function
    /// immediately returns `None`. Otherwise, it returns `Some(result)` where `result` contains
    /// the mapped elements in an array.
    ///
    /// This function does not allocate space on the heap
    ///
    /// For functions that return an `Result`, consider using [`UnarrayArrayExt::map_result`]
    fn map_option<S>(self, f: impl FnMut(T) -> Option<S>) -> Option<[S; N]>;
}

impl<T, const N: usize> UnarrayArrayExt<T, N> for [T; N] {
    fn map_result<S, E>(self, mut f: impl FnMut(T) -> Result<S, E>) -> Result<[S; N], E> {
        let mut result = uninit_buf();

        // This is quaranteed to loop over every element (or panic), since both `result` and `self` have N elements
        // If a panic occurs, uninitialized data is never dropped, since `MaybeUninit` wraps its
        // contained data in `ManuallyDrop`
        for (item, slot) in self.into_iter().zip(&mut result) {
            match f(item) {
                Ok(s) => slot.write(s),
                Err(e) => return Err(e),
            };
        }

        // SAFETY:
        // At this point in execution, we have iterated over all elements of `result`. If any
        // errors were encountered, we would have already returned. So it's safe to remove the
        // MaybeUninit wrapper
        Ok(unsafe { mark_initialized(result) })
    }

    fn map_option<S>(self, mut f: impl FnMut(T) -> Option<S>) -> Option<[S; N]> {
        // transform to a `Result`-returning function so we can avoid duplicating unsafe code
        let actual_f = |t: T| -> Result<S, ()> { f(t).ok_or(()) };

        let result: Result<[S; N], ()> = UnarrayArrayExt::map_result(self, actual_f);
        match result {
            Ok(result) => Some(result),
            Err(()) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UnarrayArrayExt;
    use crate::testing::array_strategy;
    use proptest::prelude::*;
    use test_strategy::proptest;

    #[test]
    fn test_map_option() {
        let array = [1, 2, 3];
        let result = array.map_option(|i| Some(i * 2)).unwrap();
        assert_eq!(result, [2, 4, 6]);
    }

    #[test]
    #[should_panic]
    fn test_map_option_panic() {
        let array = [1, 2, 3];
        array.map_option(|i| {
            if i > 2 {
                panic!();
            }

            Some(i)
        });
    }

    #[test]
    fn test_map_result() {
        let array = [1, 2, 3];
        let result: Result<_, ()> = array.map_result(|i| Ok(i * 2));
        assert_eq!(result.unwrap(), [2, 4, 6]);
    }

    #[test]
    #[should_panic]
    fn test_map_result_panic() {
        let array = [1, 2, 3];
        let _ = array.map_result(|i| -> Result<i32, ()> {
            if i > 2 {
                panic!();
            }

            Ok(i)
        });
    }

    const LEN: usize = 100;

    #[proptest]
    #[cfg_attr(miri, ignore)]
    fn proptest_option_map(#[strategy(array_strategy::<LEN>())] array: [String; LEN]) {
        let expected = array.iter().map(|s| s.len()).collect::<Vec<_>>();
        let expected: [usize; LEN] = expected.try_into().unwrap();
        let result = array.map_option(|s| Some(s.len()));
        prop_assert_eq!(expected, result.unwrap());
    }

    #[proptest]
    #[cfg_attr(miri, ignore)]
    fn proptest_result_map(#[strategy(array_strategy::<LEN>())] array: [String; LEN]) {
        let expected = array.iter().map(|s| s.len()).collect::<Vec<_>>();
        let expected: [usize; LEN] = expected.try_into().unwrap();
        let result: Result<_, ()> = array.map_result(|s| Ok(s.len()));
        prop_assert_eq!(expected, result.unwrap());
    }
}
