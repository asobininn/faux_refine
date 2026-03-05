use std::marker::PhantomData;

#[derive(Debug, Clone, Copy)]
pub struct Nil;

#[derive(Debug, Clone, Copy)]
pub struct Cons<H, T>(PhantomData<(H, T)>);

/// ## 展開例
/// ```
/// proofs!()                   // Nil
/// proofs!(IsOdd)              // Cons<IsOdd, Nil>
/// proofs!(IsOdd, Grearter<3>) // Cons<IsOdd, Cons<Greater<3>, Nil>>
/// ```
/// ## 使用例
/// ```
/// type OddAndGt3 = ValidedNumber<i32, proofs!(Isodd, Greater<3>)>;
/// 
/// fn odd_only(n: &ValidatedNumber<i32, proofs!(IsOdd)>) {/* */}
/// ```
#[macro_export]
macro_rules! proofs {
    // 終端
    () => { Nil };
    // 単一要素
    ($head:ty) => {
        Cons<$head, Nil>
    };
    // 複数要素
    ($head:ty, $($tail:ty), +) => {
        Cons<$head, proofs![$($tail),+]>
    };
}
