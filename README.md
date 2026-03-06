# faux_refine

Rustで依存型もどきを実現するライブラリ。  

```rust
// 制約は最小限だけ宣言すればよい。関数のシグネチャがそのまま必要なコンテキストを表現する。
fn send_email(address: &ValidatedEmail<proofs!(NonEmpty, ValidFormat)>) { ... }
fn log(address: &ValidatedEmail<proofs!(NonEmpty)>) { ... }

let email = ValidatedEmail::try_new(input)?;  // 一度だけ検証
send_email(&email);                           // そのまま渡せる
log(email.weaken_ref().unwrap());             // 制約を緩めてそのまま渡せる
```

## クイックスタート

```rust
use std::{convert::Infallible, fmt::Display, marker::PhantomData};

use faux_refine::{faux_refine_derive::Proof, predule::*};

// 1. Newtypeパターンのラッパー型を定義する
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

// 2. 制約を定義する
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

// 3. 使う
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

### constジェネリクスの順序を区別する

`Foo<const A: i32, const B: i32>`のような型において、`Foo<3, 4>`と`Foo<4, 3>`は別の制約として扱われる。

## できないこと

### 制約の意味論的な演算

`MinLength<2>`と`MinLength<3>`を合成しても`MinLength<5>`は自動導出できない。

```rust
fn concat(
    a: ValidatedString<proofs!(MinLength<2>)>,
    b: ValidatedString<proofs!(MinLength<3>)>,
) -> ValidatedString<proofs!(MinLength<5>)> {
    let value = format!("{}{}", a.inner(), b.inner());
    ValidatedString::try_new(value).unwrap()  // 本来不要なはずだが避けられない
}
```

### 順序関係をもつ制約

`MinLength<4>`は`MinLength<3>`を包含するが、両者は別の`PROOF_BIT`を持つため`weaken`は失敗する。

### 強い制約へ変換時の最小限のチェック

線形型システムを活用すれば変数に必要なチェックを最小限にできるが、このクレートではWeak → Strongの変換は`try_new`による再検証が必要となる。

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
