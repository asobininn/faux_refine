//! A crate that implements a pseudo-Refinement Type in Rust.
//! 
//! ## ⚠️ About Potential Probabilistic Bugs
//! This crate may experience two types of collisions:
//! 
//! 1. FNV-64 hash collisions (affects all constraint types)
//! 2. Collisions of the extra value (affects types with const generics)
//! 
//! Both types of collisions can only result in false positives (i.e., succeeding when they should fail).
//! 
//! Please avoid using this crate for security-critical purposes or in systems where an incorrect type conversion could be catastrophic.
//! ## Usage
//! ```rust
//! use std::{convert::Infallible, marker::PhantomData};
//!
//! use faux_refine::{faux_refine_derive::Pred, predule::*};
//! use nalgebra::{DMatrix, DVector};
//!
//! // 1. Define the Newtype pattern type
//! // required: #[repr(transparent)]
//!
//! #[repr(transparent)]
//! #[derive(Debug, Clone)]
//! struct Mat<P: Pred> {
//!     data: DMatrix<f64>,
//!     _pred: PhantomData<P>,
//! }
//!
//! unsafe impl<P: Pred> Refined for Mat<P> {
//!     type Inner = DMatrix<f64>;
//!     type Pred = P;
//!
//!     fn inner(&self) -> &Self::Inner {
//!         &self.data
//!     }
//!
//!     fn into_inner(self) -> Self::Inner {
//!         self.data
//!     }
//! }
//!
//! // 2. Define error types
//! // required: Implementation of From<Infallible>
//!
//! #[derive(Debug)]
//! enum MyError {
//!     NotSquare,
//!     IsSingular,
//!     NotPositiveDefinite,
//!     NotSymmetric,
//! }
//!
//! impl From<Infallible> for MyError {
//!     fn from(value: Infallible) -> Self {
//!         match value {}
//!     }
//! }
//!
//! // 3. Define predicates
//! // required: Manual definition of inclusion relationships
//!
//! #[derive(Pred, Debug, Clone)]
//! struct Square;
//! impl Validator<DMatrix<f64>> for Square {
//!     type Error = MyError;
//!
//!     fn validate(value: &DMatrix<f64>) -> Result<(), Self::Error> {
//!         value.is_square().then_some(()).ok_or(MyError::NotSquare)
//!     }
//! }
//!
//! #[derive(Pred, Debug, Clone)]
//! #[pred(extends(Square))]
//! struct NonSingular;
//! impl Validator<DMatrix<f64>> for NonSingular {
//!     type Error = MyError;
//!
//!     fn validate(value: &DMatrix<f64>) -> Result<(), Self::Error> {
//!         // check if (det(A) != 0)
//!         (value.clone().lu().determinant().abs() > 1e-10)
//!             .then_some(())
//!             .ok_or(MyError::IsSingular)
//!     }
//! }
//!
//! #[derive(Pred, Debug, Clone)]
//! #[pred(extends(NonSingular))]
//! struct PositiveDefinite;
//! impl Validator<DMatrix<f64>> for PositiveDefinite {
//!     type Error = MyError;
//!
//!     fn validate(value: &DMatrix<f64>) -> Result<(), Self::Error> {
//!         value
//!             .clone()
//!             .cholesky()
//!             .is_some()
//!             .then_some(())
//!             .ok_or(MyError::NotPositiveDefinite)
//!     }
//! }
//!
//! #[derive(Pred, Debug, Clone)]
//! #[pred(extends(Square))]
//! struct Symmetric;
//! impl Validator<DMatrix<f64>> for Symmetric {
//!     type Error = MyError;
//!
//!     fn validate(value: &DMatrix<f64>) -> Result<(), Self::Error> {
//!         // check if (A = A^T)
//!         ((value - value.transpose()).norm() < 1e-10)
//!             .then_some(())
//!             .ok_or(MyError::NotSymmetric)
//!     }
//! }
//!
//! // 4. Use
//! //
//!
//! fn determinant(m: Mat<preds!(Square)>) -> f64 {
//!     m.into_inner().lu().determinant()
//! }
//!
//! fn inverse(m: Mat<preds!(NonSingular)>) -> DMatrix<f64> {
//!     m.into_inner().try_inverse().unwrap()
//! }
//!
//! fn cholesky(m: Mat<preds!(PositiveDefinite)>) -> DMatrix<f64> {
//!     m.into_inner().cholesky().unwrap().l()
//! }
//!
//! fn least_squares(a: Mat<preds!(NonSingular)>, b: &DVector<f64>) -> DVector<f64> {
//!     a.into_inner().lu().solve(b).unwrap()
//! }
//!
//! fn condition_number(m: Mat<preds!(Symmetric)>) -> f64 {
//!     let eigen = m.into_inner().symmetric_eigen();
//!     let max = eigen.eigenvalues.max();
//!     let min = eigen.eigenvalues.min();
//!     max / min
//! }
//!
//! fn main() -> Result<(), MyError> {
//!     let data = DMatrix::from_row_slice(3, 3, &[4.0, 2.0, 2.0, 2.0, 5.0, 3.0, 2.0, 3.0, 6.0]);
//!     let m = Mat::<preds!(PositiveDefinite)>::try_new(data)?;
//!
//!     println!(
//!         "det  = {:.4}",
//!         determinant(m.clone().into_weaken().unwrap())
//!     );
//!     println!("inv  = {:.4}", inverse(m.clone().into_weaken().unwrap()));
//!     println!("chol = {:.4}", cholesky(m.clone()));
//!
//!     let b = DVector::from_vec(vec![1.0, 2.0, 3.0]);
//!     println!(
//!         "ls   = {}",
//!         least_squares(m.clone().into_weaken().unwrap(), &b)
//!     );
//!
//!     let sym = m.try_into_refine::<Mat<preds!(Symmetric)>>().map_err(|e| e.error)?;
//!     println!("condition number = {:.4}", condition_number(sym));
//!
//!     Ok(())
//! }
//! ```

pub use faux_refine_core;
#[cfg(feature = "derive")]
pub use faux_refine_derive;

pub mod predule {
    pub use faux_refine_core::{
        predicate::{bitset::*, list::*, validator::Validator},
        preds,
        refined::Refined,
    };
}

/*  (WIP) 未来に期待
struct IsSubset<const B: bool>;

trait IsTrue {}
impl IsTrue for IsSubset<true> {}

trait Contains<Pneed: Pred> {}

impl<PHas: Pred, PNeed: Pred> Contains<PNeed> for PHas where
    IsSubset<{ BitSet::is_subset_of(&PHas::PROOF_BIT, &PNeed::PROOF_BIT) }>: IsTrue
{
}
*/
