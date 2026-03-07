use std::{convert::Infallible, fmt::Display, marker::PhantomData};

use faux_refine::{faux_refine_derive::Pred, predule::*};

// 1. Define the Newtype pattern type
// required: #[repr(transparent)]

#[repr(transparent)]
#[derive(Debug, Clone)]
struct ValidatedInt<P: Pred> {
    value: i32,
    _proof: PhantomData<P>,
}

unsafe impl<P: Pred> Refined for ValidatedInt<P> {
    type Inner = i32;
    type Pred = P;

    fn inner(&self) -> &Self::Inner {
        &self.value
    }

    fn into_inner(self) -> Self::Inner {
        self.value
    }
}

impl<P: Pred> Display for ValidatedInt<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

// 2. Define error types
// required: Implementation of From<Infallible>

#[derive(Debug)]
enum MyError {
    IsNotOdd,
    IsNotFive,
    Below(i32),
    Convert,
    NotASubset,
}

impl From<Infallible> for MyError {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

// 3. Define predicates
// required: Manual definition of inclusion relationships

#[derive(Pred, Debug, Clone)]
struct IsOdd;
impl<T: num::Integer> Validator<T> for IsOdd {
    type Error = MyError;

    fn validate(value: &T) -> Result<(), Self::Error> {
        value.is_odd().then_some(()).ok_or(MyError::IsNotOdd)
    }
}

#[derive(Pred, Debug, Clone)]
struct Greater<const N: i32>;
impl<const N: i32, T: num::Integer + num::ToPrimitive> Validator<T> for Greater<N> {
    type Error = MyError;

    fn validate(value: &T) -> Result<(), Self::Error> {
        (value.to_i32().ok_or(MyError::Convert)? > N)
            .then_some(())
            .ok_or(MyError::Below(N))
    }
}

#[derive(Pred, Debug, Clone)]
#[pred(extends(IsOdd, Greater<1>))]
struct IsFive;
impl Validator<i32> for IsFive {
    type Error = MyError;

    fn validate(value: &i32) -> Result<(), Self::Error> {
        (value == &5).then_some(()).ok_or(MyError::IsNotFive)
    }
}

// 4. Use
//

fn odd_and_greater1_only(n: &ValidatedInt<preds!(IsOdd, Greater<1>)>) {
    println!("{} is an odd number and greater than 1.", n);
}

fn five_only(n: &ValidatedInt<preds!(IsFive)>) {
    println!("{} is 5!!.", n)
}

fn main() -> Result<(), MyError> {
    let n: ValidatedInt<preds!(IsFive)> = ValidatedInt::try_new(5)?;
    odd_and_greater1_only(n.as_weaken_ref().ok_or(MyError::NotASubset)?);
    five_only(&n);
    Ok(())
}
