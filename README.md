# faux_refine

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

## できること

### 強い制約から弱い制約へのO(1)での変換

```rust
let n: ValidatedNumber<i32, proofs!(IsOdd, Greater<3>)> = ValidatedNumber::try_new(11)?;
let n2: ValidatedNumber<i32, proofs!(IsOdd)> = n.weaken().unwrap();
```

### 複数の制約を合成する

`proof!`マクロで複数の制約を組み合わせる。

```rust
proofs!(NonEmpty, ValidFormat, DomainExists)
```

`[proof(extends(..))]`マクロで上位の制約を定義する。

```rust
#[derive(Proof)]
#[proof(extends(IsOdd, Greater<1>))]
struct IsFive;
```

### constジェネリクスの順序を区別する

`Foo<const A: i32, const B: i32>`のような型において、`Foo<3, 4>`と`Foo<4, 3>`は別の制約として扱われる。

## できないこと

### 制約パラメータの意味論的計算

`MinLength<2>`と`MinLength<3>`を合成しても`MinLength<5>`は自動導出できない。  
証明はユーザの責任で行う必要がある。

```rust
fn concat(
    a: ValidatedString<proofs!(MinLength<2>)>,
    b: ValidatedString<proofs!(MinLength<3>)>,
) -> ValidatedString<proofs!(MinLength<5>)> {
    let value = format!("{}{}", a.inner(), b.inner());
    ValidatedString::try_new(value).unwrap()  // 型レベルで保証できない
}
```

### 順序関係をもつ制約

`MinLength<4>`は`MinLength<3>`を包含するが、両者は別の`PROOF_BIT`を持つため`weaken`は失敗する。

### 強い制約へ変換時の最小限のチェック

線形型システムを活用すれば変数に必要なチェックを最小限にできるが、このクレートではWeak → Strongの変換は`new_unchecked`による`unsafe`なものか、`try_new`による再検証が必要となる。

## 将来できること(`generic_const_exprs`の安定化待ち)

### `Contains`トレイトによる`impl`の自然な継承

```rust
impl<P: Proof> ValidatedString<P>
where
    P: Contains<proofs!(NonEmpty)>, 
{
    fn foo(&self) { ... }  
}

let s: ValidatedString<proofs!(NonEmpty, MinLength<2>)> = ...;
s.foo(); 
```

### `proofs!`の順序が無関係になる

## ⚠️ 確率的なバグの可能性

このクレートは2種類の衝突が起きる可能性がある。

1. FNV-64ハッシュの衝突 (全制約型)
2. `extra`値の衝突 (constジェネリクスを持つ型)

どちらの衝突も`weaken`/`weaken_ref`が誤って成功する方向にのみ働く。  

セキュリティ用途や、誤った型変換が致命的になるシステムでの使用は避けてください。

## ライセンス

MIT
