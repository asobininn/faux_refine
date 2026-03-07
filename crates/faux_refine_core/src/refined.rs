use crate::predicate::{bitset::Pred, validator::*};

/// 「ある制約を満たすことが保証された値」を表すトレイト。
/// ## 実装者が守るべき制約
/// 1. 構造が`#[repr(transparent)]`可能であること
/// ## 実装例
/// ```rust
/// use std::marker::PhantomData;
/// use faux_refine::{faux_refine_derive::Pred, predule::*};
///
/// #[repr(transparent)]
/// #[derive(Debug, Clone)]
/// struct ValidatedNumber<T, P: Pred> {
///     value: T,
///     _proof: PhantomData<P>,  // PhantomDataはレイアウトに影響しない
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

    fn weaken_ref<Target>(&self) -> Option<&Target>
    where
        Target: Refined<Inner = Self::Inner>,
    {
        Target::Pred::PRED_BIT
            .is_subset_of(&Self::Pred::PRED_BIT)
            .then_some(unsafe { &*(self as *const Self as *const Target) })
    }

    fn weaken<Target>(self) -> Result<Target, Self>
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

    fn try_refine_ref<Target>(
        &self,
    ) -> Result<&Target, <Target::Pred as ValidatorRemaining<Self::Pred, Self::Inner>>::Error>
    where
        Target: Refined<Inner = Self::Inner>,
        Target::Pred: ValidatorRemaining<Self::Pred, Self::Inner>,
    {
        Target::Pred::validate_remaining(&self.inner())
            .map(|_| unsafe { &*(self as *const Self as *const Target) })
    }

    fn try_refine<Target>(
        self,
    ) -> Result<
        Target,
        (
            Self,
            <Target::Pred as ValidatorRemaining<Self::Pred, Self::Inner>>::Error,
        ),
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
            Err(e) => Err((self, e)),
        }
    }
}
