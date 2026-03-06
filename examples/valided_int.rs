use std::{convert::Infallible, fmt::Display, marker::PhantomData};

use faux_refine::{faux_refine_derive::Proof, predule::*};

// 検証済みの数値を表すNewType
#[repr(transparent)]
#[derive(Debug, Clone)]
struct ValidatedInt<P: Proof> {
    value: i32,
    _proof: PhantomData<P>,
}

unsafe impl<P: Proof> Refined for ValidatedInt<P> {
    type Inner = i32;
    type Proof = P;

    fn inner(&self) -> &Self::Inner {
        &self.value
    }

    fn into_inner(self) -> Self::Inner {
        self.value
    }
}

impl<P: Proof> Display for ValidatedInt<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

// エラー型
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

#[derive(Debug, Clone, Proof)]
#[proof(extends(IsOdd, Greater<1>))]
struct IsFive;
impl Validator<i32> for IsFive {
    type Error = MyError;

    fn validate(value: &i32) -> Result<(), Self::Error> {
        (value == &5).then_some(()).ok_or(MyError::IsNotFive)
    }
}

// 使用例
fn odd_and_greater1_only(n: &ValidatedInt<proofs!(IsOdd, Greater<1>)>) {
    println!("{} is an odd number and greater than 1.", n);
}

fn five_only(n: &ValidatedInt<proofs!(IsFive)>) {
    println!("{} is 5!!.", n)
}

fn main() -> Result<(), MyError> {
    let n: ValidatedInt<proofs!(IsFive)> = ValidatedInt::try_new(5)?;
    odd_and_greater1_only(n.weaken_ref().ok_or(MyError::NotASubset)?);
    five_only(&n);
    Ok(())
}
