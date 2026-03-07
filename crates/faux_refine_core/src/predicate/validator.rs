use std::convert::Infallible;

use crate::predicate::{
    bitset::Pred,
    list::{Cons, Nil},
};

/// A trait for validating at runtime that a value of type `T` satisfies a constraint.
/// ## Examples
/// ```
/// #[derive(Pred, Debug, Clone)]
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
    /// Error type returned when verification fails.
    /// 
    /// required: Implementation of [`From<Infallible>`]
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

/// A differential validation trait used internally by `Refined::try_into_refine`.
/// This trait is not intended to be implemented directly by users.
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
