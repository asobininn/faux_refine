# faux-refine

A crate that implements a pseudo-Refinement Type in Rust.

``` cargo
cargo add faux-refine -F "derive"
```

``` toml
[dependencies]
faux_refine = { version = "0.1" features = ["derive"] }
```

## Usage

```rust
use std::{convert::Infallible, marker::PhantomData};

use faux_refine::{faux_refine_derive::Pred, predule::*};
use nalgebra::{DMatrix, DVector};

// 1. Define the Newtype pattern type
// required: #[repr(transparent)]

#[repr(transparent)]
#[derive(Debug, Clone)]
struct Mat<P: Pred> {
    data: DMatrix<f64>,
    _pred: PhantomData<P>,
}

unsafe impl<P: Pred> Refined for Mat<P> {
    type Inner = DMatrix<f64>;
    type Pred = P;

    fn inner(&self) -> &Self::Inner {
        &self.data
    }

    fn into_inner(self) -> Self::Inner {
        self.data
    }
}

// 2. Define error types
// required: Implementation of From<Infallible>

#[derive(Debug)]
enum MyError {
    NotSquare,
    IsSingular,
    NotPositiveDefinite,
    NotSymmetric,
}

impl From<Infallible> for MyError {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

// 3. Define predicates
// required: Manual definition of inclusion relationships

#[derive(Pred, Debug, Clone)]
struct Square;
impl Validator<DMatrix<f64>> for Square {
    type Error = MyError;

    fn validate(value: &DMatrix<f64>) -> Result<(), Self::Error> {
        value.is_square().then_some(()).ok_or(MyError::NotSquare)
    }
}

#[derive(Pred, Debug, Clone)]
#[pred(extends(Square))]
struct NonSingular;
impl Validator<DMatrix<f64>> for NonSingular {
    type Error = MyError;

    fn validate(value: &DMatrix<f64>) -> Result<(), Self::Error> {
        // check if (det(A) != 0)
        (value.clone().lu().determinant().abs() > 1e-10)
            .then_some(())
            .ok_or(MyError::IsSingular)
    }
}

#[derive(Pred, Debug, Clone)]
#[pred(extends(NonSingular))]
struct PositiveDefinite;
impl Validator<DMatrix<f64>> for PositiveDefinite {
    type Error = MyError;

    fn validate(value: &DMatrix<f64>) -> Result<(), Self::Error> {
        value
            .clone()
            .cholesky()
            .is_some()
            .then_some(())
            .ok_or(MyError::NotPositiveDefinite)
    }
}

#[derive(Pred, Debug, Clone)]
#[pred(extends(Square))]
struct Symmetric;
impl Validator<DMatrix<f64>> for Symmetric {
    type Error = MyError;

    fn validate(value: &DMatrix<f64>) -> Result<(), Self::Error> {
        // check if (A = A^T)
        ((value - value.transpose()).norm() < 1e-10)
            .then_some(())
            .ok_or(MyError::NotSymmetric)
    }
}

// 4. Use
//

fn determinant(m: Mat<preds!(Square)>) -> f64 {
    m.into_inner().lu().determinant()
}

fn inverse(m: Mat<preds!(NonSingular)>) -> DMatrix<f64> {
    m.into_inner().try_inverse().unwrap()
}

fn cholesky(m: Mat<preds!(PositiveDefinite)>) -> DMatrix<f64> {
    m.into_inner().cholesky().unwrap().l()
}

fn least_squares(a: Mat<preds!(NonSingular)>, b: &DVector<f64>) -> DVector<f64> {
    a.into_inner().lu().solve(b).unwrap()
}

fn condition_number(m: Mat<preds!(Symmetric)>) -> f64 {
    let eigen = m.into_inner().symmetric_eigen();
    let max = eigen.eigenvalues.max();
    let min = eigen.eigenvalues.min();
    max / min
}

fn main() -> Result<(), MyError> {
    let data = DMatrix::from_row_slice(3, 3, &[4.0, 2.0, 2.0, 2.0, 5.0, 3.0, 2.0, 3.0, 6.0]);
    let m = Mat::<preds!(PositiveDefinite)>::try_new(data)?;

    println!(
        "det  = {:.4}",
        determinant(m.clone().into_weaken().unwrap())
    );
    println!("inv  = {:.4}", inverse(m.clone().into_weaken().unwrap()));
    println!("chol = {:.4}", cholesky(m.clone()));

    let b = DVector::from_vec(vec![1.0, 2.0, 3.0]);
    println!(
        "ls   = {}",
        least_squares(m.clone().into_weaken().unwrap(), &b)
    );

    let sym = m.try_into_refine::<Mat<preds!(Symmetric)>>().map_err(|e| e.error)?;
    println!("condition number = {:.4}", condition_number(sym));

    Ok(())
}
```

## 特徴

### O(1) での Strong → Weak 変換

```rust
let n: ValidatedInt<preds!(IsOdd, Greater<3>)> = ValidatedInt::try_new(11)?;
// ビット比較のみ
let n: ValidatedInt<preds!(IsOdd)> = n.into_weaken().unwrap();
```

### 重複チェックなしの Weak → Strong 変換

```rust
let n: ValidatedInt<preds!(IsOdd)> = ValidatedInt::try_new(11)?;
// IsGreater<3>のみを確認する
let n: ValidatedInt<preds!(IsOdd, Greater<3>)> = n.try_into_refine().map_err(|e| e.error)?;
```

### 複数の制約を合成する

`preds!`マクロで複数の制約を組み合わせる。

```rust
preds!(NonEmpty, ValidFormat, DomainExists)
```

`[pred(extends(..))]`マクロで制約を継承する。

```rust
#[derive(Preds)]
#[pred(extends(IsOdd, Greater<1>))]
struct IsFive;
```

### const ジェネリクスの順序区別

`Foo<const A: i32, const B: i32>`のような型において、`Foo<3, 4>`と`Foo<4, 3>`は別の制約として扱われる。

## 制限事項

### `preds!` の順序

`T<preds!(A, B)>` と `T<preds!(B, A)>` は別の型として扱われるため、そのままでは渡せません。
`weaken` / `refine` を使えば同強度の制約間の変換は可能です。

`generic_const_exprs` の安定化後に `Contains` トレイトとして対応予定です。

```rust
let n = ValidatedInt::<preds!(IsNat, IsOdd)>::try_new(11)?;
let rn: &ValidatedInt<preds!(IsOdd, IsNat)> = &n.as_weaken_ref().unwrap();
let n: ValidatedInt<preds!(IsOdd, IsNat)> = n.try_into_refine().unwrap();
```

### 制約の意味論的な演算・包含関係

このライブラリは「どの制約を持つか」をビットセットで管理しますが、
制約同士の意味論的な関係を知りません。

#### 演算の自動証明が不可能

奇数同士を足すと偶数になることは数学的に自明ですが、
ライブラリはその意味を知らないため`unwrap`が必要になります。

```rust
// IsOdd かつ IsPositive な数を足したら IsEven になるはずだが導出できない
fn add_odd_positives(
    a: ValidatedInt<preds!(IsOdd, IsPositive)>,
    b: ValidatedInt<preds!(IsOdd, IsPositive)>,
) -> ValidatedInt<preds!(IsEven)> {
    // 自明なはずだが避けられない
    ValidatedInt::try_new(a.into_inner() + b.into_inner()).unwrap() 
}
```

#### 順序関係の包含

`InRange<0, 10>`は意味的には`InRange<0, 20>`を包含しますが、
const ジェネリクスの大小関係を型システムで表現できないため`weaken`が失敗します。

```rust
let n: ValidatedInt<preds!(InRange<0, 10>)> = ValidatedInt::try_new(5)?;
assert!(
    n.into_weaken::<ValidatedInt<preds!(InRange<0, 20>)>>()
        .is_err()
);
```

## 将来の展望(`generic_const_exprs`の安定化待ち)

### `Contains`トレイトによる`impl`の上位制約への実装

```rust
impl<P: Pred> ValidatedString<P>
where
    P: Contains<preds!(NonEmpty)>, 
{
    fn foo(&self) { ... }  
}

let s: ValidatedString<preds!(NonEmpty, MinLength<2>)> = ...;
s.foo(); 
```

### `preds!`の順序が無関係になる

## ⚠️ About Potential Probabilistic Bugs

This crate may experience two types of collisions:

1. FNV-64 hash collisions (affects all constraint types)
2. Collisions of the extra value (affects types with const generics)

Both types of collisions can only result in false positives (i.e., succeeding when they should fail).

Please avoid using this crate for security-critical purposes or in systems where an incorrect type conversion could be catastrophic.

## License

MIT license
