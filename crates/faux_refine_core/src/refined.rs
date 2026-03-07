use crate::predicate::{
    bitset::Pred,
    validator::{Validator, ValidatorRemaining},
};

/// Error type returned when [`Refined::try_into_refine`] fails.
#[derive(Debug, Clone)]
pub struct RefineError<T, E> {
    pub value: T,
    pub error: E,
}

/// A trait representing values guaranteed to satisfy a constraint.
///
/// required: #\[repr(transparent)]
/// ## Examples
/// ```rust
/// use std::marker::PhantomData;
/// use faux_refine::{faux_refine_derive::Pred, predule::*};
///
/// #[repr(transparent)]
/// #[derive(Debug, Clone)]
/// struct ValidatedNumber<T, P: Pred> {
///     value: T,
///     _proof: PhantomData<P>,
/// }
///
/// unsafe impl<T, P: Pred> Refined for ValidatedNumber<T, P> {
///     type Inner = T;
///     type Pred = P;
///
///     fn inner(&self) -> &T { &self.value }
///     fn into_inner(self) -> T { self.value }
/// }
/// ```
pub unsafe trait Refined: Sized {
    type Inner;
    type Pred: Pred;

    fn inner(&self) -> &Self::Inner;
    fn into_inner(self) -> Self::Inner;

    /// Constructs Self by validating the value and ensuring that all constraints are satisfied.
    /// ## Errors
    /// Returns the error from the first failing constraint during the validation of `Self::Proof`
    /// ## Examples
    /// ```rust
    /// let n = ValidatedInt::<preds!(IsOdd, Greater<3>)>::try_new(11);
    /// assert!(n.is_ok());
    ///
    /// let n = ValidatedInt::<preds!(IsOdd)>::try_new(4);
    /// assert!(n.is_err());
    /// ```
    fn try_new(value: Self::Inner) -> Result<Self, <Self::Pred as Validator<Self::Inner>>::Error>
    where
        Self::Pred: Validator<Self::Inner>,
    {
        Self::Pred::validate(&value).map(|_| unsafe {
            let mut slot = std::mem::MaybeUninit::<Self>::uninit();
            std::ptr::write(slot.as_mut_ptr() as *mut Self::Inner, value);
            slot.assume_init()
        })
    }

    unsafe fn new_unchecked(value: Self::Inner) -> Self {
        unsafe {
            let mut slot = std::mem::MaybeUninit::<Self>::uninit();
            std::ptr::write(slot.as_mut_ptr() as *mut Self::Inner, value);
            slot.assume_init()
        }
    }

    /// **Strong → Weak**: Reinterprets the reference as a type requiring fewer constraints.
    /// ## Order
    /// 256-bit bitwise comparison **O(1)**.
    /// ## Examples
    /// ```rust
    /// let n = ValidatedInt::<preds!(IsOdd, Greater<3>)>::try_new(11)?;
    /// let rn = n.as_weaken_ref::<ValidatedInt<preds!(IsOdd)>>();
    /// assert!(rn.is_some());
    ///
    /// let n = ValidatedInt::<preds!(IsOdd)>::try_new(5)?;
    /// let rn = n.as_weaken_ref::<ValidatedInt<preds!(IsFive)>>();
    /// assert!(rn.is_none());
    /// ```
    fn as_weaken_ref<Target>(&self) -> Option<&Target>
    where
        Target: Refined<Inner = Self::Inner>,
    {
        Target::Pred::PRED_BIT
            .is_subset_of(&Self::Pred::PRED_BIT)
            .then_some(unsafe { &*(self as *const Self as *const Target) })
    }

    /// **Strong → Weak**: Converts into a type requiring fewer constraints (moves ownership).
    /// ## Order
    /// 256-bit bitwise comparison **O(1)**.
    /// ## Examples
    /// ```rust
    /// let n = ValidatedInt::<preds!(IsOdd, Greater<3>)>::try_new(11)?;
    /// let new_n = n.into_weaken::<ValidatedInt<preds!(IsOdd)>>();
    /// assert!(new_n.is_ok());
    ///
    /// let n = ValidatedInt::<preds!(IsOdd)>::try_new(5)?;
    /// let new_n = n.into_weaken::<ValidatedInt<preds!(IsFive)>>();
    /// assert!(new_n.is_err());
    /// ```
    fn into_weaken<Target>(self) -> Result<Target, Self>
    where
        Target: Refined<Inner = Self::Inner>,
    {
        if Target::Pred::PRED_BIT.is_subset_of(&Self::Pred::PRED_BIT) {
            Ok(unsafe {
                let value = std::mem::ManuallyDrop::new(self);
                std::ptr::read(value.inner() as *const Self::Inner as *const Target)
            })
        } else {
            Err(self)
        }
    }

    /// **Weak → Strong**: Promotes the reference to a type requiring more constraints.
    ///
    /// Checks for constraints already guaranteed by Self are skipped,
    /// and only the additional constraints are validated at runtime.
    ///
    /// ## Returns
    /// - `Ok(&Target)` — When the difference constraint is satisfied.
    /// - `Err(Error)` — Returns the error from the first failing constraint during the validation of `Self::Proof`.
    /// ## Examples
    /// ```rust 
    /// let n = ValidatedInt::<preds!(IsOdd)>::try_new(5)?;
    /// let rn = n.try_as_refine_ref::<ValidatedInt<preds!(IsFive)>>();
    /// assert!(rn.is_ok());
    ///
    /// let n = ValidatedInt::<preds!(IsOdd)>::try_new(1)?;
    /// let rn = n.try_as_refine_ref::<ValidatedInt<preds!(IsFive)>>();
    /// assert!(rn.is_err());
    /// ```
    fn try_as_refine_ref<Target>(
        &self,
    ) -> Result<&Target, <Target::Pred as ValidatorRemaining<Self::Pred, Self::Inner>>::Error>
    where
        Target: Refined<Inner = Self::Inner>,
        Target::Pred: ValidatorRemaining<Self::Pred, Self::Inner>,
    {
        Target::Pred::validate_remaining(&self.inner())
            .map(|_| unsafe { &*(self as *const Self as *const Target) })
    }

    /// **Weak → Strong**: Promotes into a type requiring more constraints (moves ownership).
    ///
    /// Checks for constraints already guaranteed by Self are skipped,
    /// and only the additional constraints are validated at runtime.
    ///
    /// ## Returns
    /// - `Ok(Target)` — When the difference constraint is satisfied.
    /// - `Err(RefineError(Self, Error))` — (Original value, Returns the error from the first failing constraint during the validation of `Self::Proof`)
    /// ## Examples
    /// ```rust
    /// let n = ValidatedInt::<preds!(IsOdd)>::try_new(5)?;
    /// let new_n = n.try_into_refine::<ValidatedInt<preds!(IsFive)>>();
    /// assert!(new_n.is_ok());
    /// 
    /// let n = ValidatedInt::<preds!(IsOdd)>::try_new(1)?;
    /// let new_n = n.try_into_refine::<ValidatedInt<preds!(IsFive)>>();
    /// assert!(new_n.is_err());
    /// ```
    fn try_into_refine<Target>(
        self,
    ) -> Result<
        Target,
        RefineError<Self, <Target::Pred as ValidatorRemaining<Self::Pred, Self::Inner>>::Error>,
    >
    where
        Target: Refined<Inner = Self::Inner>,
        Target::Pred: ValidatorRemaining<Self::Pred, Self::Inner>,
    {
        match Target::Pred::validate_remaining(&self.inner()) {
            Ok(()) => {
                let value = std::mem::ManuallyDrop::new(self);
                Ok(unsafe { std::ptr::read(value.inner() as *const Self::Inner as *const Target) })
            }
            Err(error) => Err(RefineError { value: self, error }),
        }
    }
}
