use std::{convert::Infallible, fmt::Display, marker::PhantomData};

use faux_refine::{faux_refine_derive::Proof, predule::*};

// 検証済みの数値を表すNewType
#[repr(transparent)]
#[derive(Debug, Clone)]
struct ValidatedNumber<T, P: Proof> {
    value: T,
    _proof: PhantomData<P>,
}

unsafe impl<T, P: Proof> Refined for ValidatedNumber<T, P> {
    type Inner = T;
    type Proof = P;

    fn inner(&self) -> &Self::Inner {
        &self.value
    }

    fn into_inner(self) -> Self::Inner {
        self.value
    }
}

impl<T: Display, P: Proof> Display for ValidatedNumber<T, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

// エラー型
#[derive(Debug)]
enum MyError {
    IsNotOdd,
    Below(i32),
    Convert,
    NotASubset,
}

impl From<Infallible> for MyError {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

// -- 制約群 -----
#[derive(Debug, Clone, Proof)]
struct IsOdd;
impl<T: num::Integer> Validator<T> for IsOdd {
    type Error = MyError;

    fn validate(value: &T) -> Result<(), Self::Error> {
        value.is_odd().then_some(()).ok_or(MyError::IsNotOdd)
    }
}

#[derive(Debug, Clone, Proof)]
struct Greater<const N: i32>;
impl<const N: i32, T: num::Integer + num::ToPrimitive> Validator<T> for Greater<N> {
    type Error = MyError;

    fn validate(value: &T) -> Result<(), Self::Error> {
        (value.to_i32().ok_or(MyError::Convert)? > N)
            .then_some(())
            .ok_or(MyError::Below(N))
    }
}

// 使用例
fn odd_only<T: Display>(n: &ValidatedNumber<T, proofs!(IsOdd)>) {
    println!("{} is odd number.", n);
}

fn odd_and_greater3_only<T: Display>(n: &ValidatedNumber<T, proofs!(IsOdd, Greater<3>)>) {
    println!("{} is odd and greater than 3.", n)
}

fn main() -> Result<(), MyError> {
    let n = ValidatedNumber::try_new(11)?;
    odd_only(n.weaken_ref().ok_or(MyError::NotASubset)?);
    odd_and_greater3_only(&n);
    Ok(())
}
