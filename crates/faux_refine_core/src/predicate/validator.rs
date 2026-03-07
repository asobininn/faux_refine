use std::convert::Infallible;

use crate::predicate::{
    bitset::Pred,
    list::{Cons, Nil},
};

/// 型 `T` の値が制約を満たすかを実行時に検証するトレイト。
/// ## 使用例
/// ```
/// #[derive(Debug, Clone, Pred)]
/// struct IsOdd;
///
/// impl<T: num::Integer> Validator<T> for IsOdd {
///     type Error = MyError;
///
///     fn validate(value: &T) -> Result<(), Self::Error> {
///         value.is_odd().then_some(()).ok_or(MyError::IsNotOdd)
///     }
/// }
/// ```
pub trait Validator<T> {
    /// 検証失敗時に返すエラー型。<br>
    /// [`From<Inallible>`] の実装が必要
    type Error: From<Infallible>;

    fn validate(value: &T) -> Result<(), Self::Error>;
}

impl<T> Validator<T> for Nil {
    type Error = Infallible;

    fn validate(_value: &T) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<V, Rest, T, E> Validator<T> for Cons<V, Rest>
where
    V: Validator<T, Error = E>,
    Rest: Validator<T>,
    E: From<Rest::Error> + From<Infallible>,
{
    type Error = E;

    fn validate(value: &T) -> Result<(), Self::Error> {
        V::validate(value)?;
        Rest::validate(value).map_err(E::from)
    }
}

pub trait ValidatorRemaining<PHas: Pred, T> {
    type Error;

    fn validate_remaining(value: &T) -> Result<(), Self::Error>;
}

impl<PHas: Pred, T> ValidatorRemaining<PHas, T> for Nil {
    type Error = Infallible;

    fn validate_remaining(_value: &T) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<V: Pred, Rest, T, E, PHas: Pred> ValidatorRemaining<PHas, T> for Cons<V, Rest>
where
    V: Validator<T, Error = E> + Pred,
    Rest: ValidatorRemaining<PHas, T>,
    E: From<Rest::Error> + From<Infallible>,
{
    type Error = E;

    fn validate_remaining(value: &T) -> Result<(), Self::Error> {
        if V::PRED_BIT.is_subset_of(&PHas::PRED_BIT) {
            Rest::validate_remaining(value).map_err(E::from)
        } else {
            V::validate(value)?;
            Rest::validate_remaining(value).map_err(E::from)
        }
    }
}
