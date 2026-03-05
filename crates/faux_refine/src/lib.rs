//! Rustで依存型もどきを実現するクレート
//! ### ⚠️ 確率的なバグの可能性について
//! 制約の包含関係の判定は256bitのBloom filterによって行うため、
//! 低確率ではあるが包含関係がないのに`true`を返す可能性がある<br>
//! セキュリティ用途や、誤った型変換が致命的になるシステムでの使用は避けてください。
//! ## 使用例
//! ```rust
//! use std::{convert::Infallible, fmt::Display, marker::PhantomData};
//!
//! use faux_refine::{faux_refine_derive::Proof, purelude::*};
//!
//! // 検証済みの数値を表すNewType
//! #[repr(transparent)]
//! #[derive(Debug, Clone)]
//! struct ValidedNumber<T, P: Proof> {
//!     value: T,
//!     _proof: PhantomData<P>,
//! }
//!
//! unsafe impl<T, P: Proof> Refined for ValidedNumber<T, P> {
//!     type Inner = T;
//!     type Proof = P;
//!
//!     fn inner(&self) -> &Self::Inner {
//!         &self.value
//!     }
//!
//!     fn into_inner(self) -> Self::Inner {
//!         self.value
//!     }
//! }
//!
//! impl<T: Display, P: Proof> Display for ValidedNumber<T, P> {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         write!(f, "{}", self.value)
//!     }
//! }
//!
//! // エラー型
//! #[derive(Debug)]
//! enum MyError {
//!     IsNotOdd,
//!     Below(i32),
//!     Convert,
//!     NotASubset,
//! }
//!
//! impl From<Infallible> for MyError {
//!     fn from(value: Infallible) -> Self {
//!         match value {}
//!     }
//! }
//!
//! // -- 制約群 -----
//! #[derive(Debug, Clone, Proof)]
//! struct IsOdd;
//! impl<T: num::Integer> Validator<T> for IsOdd {
//!     type Error = MyError;
//!
//!     fn validate(value: &T) -> Result<(), Self::Error> {
//!         value.is_odd().then_some(()).ok_or(MyError::IsNotOdd)
//!     }
//! }
//!
//! #[derive(Debug, Clone, Proof)]
//! struct Greater<const N: i32>;
//! impl<const N: i32, T: num::Integer + num::ToPrimitive> Validator<T> for Greater<N> {
//!     type Error = MyError;
//!
//!     fn validate(value: &T) -> Result<(), Self::Error> {
//!         (value.to_i32().ok_or(MyError::Convert)? > N)
//!             .then_some(())
//!             .ok_or(MyError::Below(N))
//!     }
//! }
//!
//! // 使用例
//! fn odd_only<T: Display>(n: &ValidedNumber<T, proofs!(IsOdd)>) {
//!     println!("{} is odd number.", n);
//! }
//!
//! fn odd_and_greater3_only<T: Display>(n: &ValidedNumber<T, proofs!(IsOdd, Greater<3>)>) {
//!     println!("{} is odd and greater than 3.", n)
//! }
//!
//! fn main() -> Result<(), MyError> {
//!     let n = ValidedNumber::try_new(11)?;
//!     odd_only(n.weaken_ref().ok_or(MyError::NotASubset)?);
//!     odd_and_greater3_only(&n);
//!     Ok(())
//! }
//! ```

pub use faux_refine_core;
#[cfg(feature = "derives")]
pub use faux_refine_derive;

pub mod purelude {
    pub use faux_refine_core::{
        proof::{bitset::*, list::*, validator::Validator},
        proofs,
        refined::Refined,
    };
}

/*  TODO: 未来に期待
struct IsSubset<const B: bool>;

trait IsTrue {}
impl IsTrue for IsSubset<true> {}

trait Contains<Pneed: Proof> {}

impl<PHas: Proof, PNeed: Proof> Contains<PNeed> for PHas where
    IsSubset<{ BitSet::is_subset_of(&PHas::PROOF_BIT, &PNeed::PROOF_BIT) }>: IsTrue
{
}
*/
