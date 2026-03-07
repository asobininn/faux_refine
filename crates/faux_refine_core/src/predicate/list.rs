use std::marker::PhantomData;

#[derive(Debug, Clone, Copy)]
pub struct Nil;

#[derive(Debug, Clone, Copy)]
pub struct Cons<H, T>(PhantomData<(H, T)>);

/// ## Example Layout
/// ```
/// preds!()                   // Nil
/// preds!(IsOdd)              // Cons<IsOdd, Nil>
/// preds!(IsOdd, Grearter<3>) // Cons<IsOdd, Cons<Greater<3>, Nil>>
/// ```
/// ## Examples
/// ```
/// type OddAndGt3 = ValidedNumber<i32, preds!(Isodd, Greater<3>)>;
/// 
/// fn odd_only(n: &ValidatedNumber<i32, preds!(IsOdd)>) {/* */}
/// ```
#[macro_export]
macro_rules! preds {
    // 終端
    () => { Nil };
    // 単一要素
    ($head:ty) => {
        Cons<$head, Nil>
    };
    // 複数要素
    ($head:ty, $($tail:ty), +) => {
        Cons<$head, preds![$($tail),+]>
    };
}
