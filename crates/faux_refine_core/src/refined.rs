use crate::proof::{bitset::Proof, validator::Validator};

/// 「ある制約を満たすことが保証された値」を表すトレイト。
/// ## 実装者が守るべき制約
/// 1. 構造が`#[repr(transparent)]`可能であること
/// ## 実装例
/// ```rust
/// use std::marker::PhantomData;
/// use faux_refine::{faux_refine_derive::Proof, predule::*};
///
/// #[repr(transparent)]
/// #[derive(Debug, Clone)]
/// struct ValidatedNumber<T, P: Proof> {
///     value: T,
///     _proof: PhantomData<P>,  // PhantomDataはレイアウトに影響しない
/// }
///
/// unsafe impl<T, P: Proof> Refined for ValidatedNumber<T, P> {
///     type Inner = T;
///     type Proof = P;
///
///     fn inner(&self) -> &T { &self.value }
///     fn into_inner(self) -> T { self.value }
/// }
/// ```
pub unsafe trait Refined: Sized {
    type Inner;
    type Proof: Proof;

    fn inner(&self) -> &Self::Inner;
    fn into_inner(self) -> Self::Inner;

    fn try_new(value: Self::Inner) -> Result<Self, <Self::Proof as Validator<Self::Inner>>::Error>
    where
        Self::Proof: Validator<Self::Inner>,
    {
        Self::Proof::validate(&value).map(|_| unsafe {
            let mut slot = std::mem::MaybeUninit::<Self>::uninit();
            std::ptr::write(slot.as_mut_ptr() as *mut Self::Inner, value);
            slot.assume_init()
        })
    }

    fn weaken_ref<Target>(&self) -> Option<&Target>
    where
        Target: Refined<Inner = Self::Inner>,
    {
        Target::Proof::PROOF_BIT
            .is_subset_of(&Self::Proof::PROOF_BIT)
            .then_some(unsafe { &*(self as *const Self as *const Target) })
    }

    fn weaken<Target>(self) -> Result<Target, Self>
    where
        Target: Refined<Inner = Self::Inner>,
    {
        if Target::Proof::PROOF_BIT.is_subset_of(&Self::Proof::PROOF_BIT) {
            Ok(unsafe {
                let value = std::mem::ManuallyDrop::new(self);
                std::ptr::read(value.inner() as *const Self::Inner as *const Target)
            })
        } else {
            Err(self)
        }
    }
}
