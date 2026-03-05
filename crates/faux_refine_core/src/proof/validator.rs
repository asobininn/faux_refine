use std::convert::Infallible;

use crate::proof::list::{Cons, Nil};

/// 型 `T` の値が制約を満たすかを実行時に検証するトレイト  
/// ## 使用例
/// ```
/// #[derive(Debug, Clone, Proof)]
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
    /// 検証失敗時に返すエラー型<br>
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