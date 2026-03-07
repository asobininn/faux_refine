use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, Token, Type, parse::Parse, parse_macro_input, punctuated::Punctuated};

// #[Pred(extends(..))]のパース

struct PredAttr {
    extends: Vec<Type>,
}

impl Parse for PredAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // 先読みして"extends"か確認してからパースする
        if !input.peek(syn::Ident) {
            return Err(input.error("expected `extends(..)`"));
        }
        let keyword: syn::Ident = input.parse()?;
        if keyword != "extends" {
            return Err(syn::Error::new(
                keyword.span(),
                format!("unknown option `{keyword}`, expected `extends(..)`"),
            ));
        }
        let content;
        syn::parenthesized!(content in input);
        let types = Punctuated::<Type, Token![,]>::parse_terminated(&content)?;
        Ok(PredAttr {
            extends: types.into_iter().collect(),
        })
    }
}

// #[derive(Pred)]の実装

/// 制約型に[`Pred`]を自動実装するderiveマクロ。<br>
/// 型の名前とconstジェネリクスパラメータから`BitSet`を定数計算で生成し、
/// `Pred::PRED_BIT`として実装する。
/// ## 実装例
/// ```rust
/// #[derive(Pred)]
/// struct IsOdd;
/// 
/// #[derive(Pred)]
/// #[proofs(extends(IsNat, IsOdd))]
/// struct IsOne;
/// ```
/// ## `extends`による制約の継承
/// `#[Pred(extends(..))]`を付与すると、親制約の`PRED_BIT`をORで合算する。<br>
/// これにより`weaken`/`weaken_ref`で親制約への変換が成立する。
/// 
/// ## ⚠️ 確率的な誤判定の可能性(2種類)
/// 1. FNV-64ハッシュの衝突 (全制約型)
/// 2. `extra`値の衝突 (constジェネリクスを持つ型)
/// 
/// どちらの衝突も`weaken`/`weaken_ref`が誤って成功する方向にのみ働く。 
#[proc_macro_derive(Pred, attributes(pred))]
pub fn derive_validator_proof(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let name = &input.ident;
    let name_str = name.to_string();
    let const_params: Vec<_> = input.generics.const_params().collect();

    // #[pred(extends(..))]をパースする
    let extends: Vec<Type> = {
        let mut result = Vec::new();
        for attr in input.attrs.iter().filter(|a| a.path().is_ident("Pred")) {
            match attr.parse_args::<PredAttr>() {
                Ok(parsed) => result.extend(parsed.extends),
                Err(e) => return e.into_compile_error().into(),
            }
        }
        result
    };
    // extendsがある場合、親制約のPRED_BITをORで合算するトークンを生成する
    // e.g. extends(IsNat, GreaterEq<1>) →
    //      | <IsNat as Pred>::PRED_BIT.bits[0] | <GreaterEq<1> as Pred>::PRED_BIT.bits[0]
    let extends_bits: Vec<TokenStream2> = (0..4)
        .map(|i| {
            let idx = syn::Index::from(i);
            quote! {
                #( | <#extends as Pred>::PRED_BIT.bits[#idx] )*
            }
        })
        .collect();
    let [eb0, eb1, eb2, eb3] = extends_bits.as_slice() else {
        unreachable!()
    };

    let bit_expr = if const_params.is_empty() {
        // 通常の型
        quote! {
            BitSet {
                bits: [
                    fnv64_seed(#name_str, SEEDS[0]) #eb0,
                    fnv64_seed(#name_str, SEEDS[1]) #eb1,
                    fnv64_seed(#name_str, SEEDS[2]) #eb2,
                    fnv64_seed(#name_str, SEEDS[3]) #eb3,
                ]
            }
        }
    } else {
        // const genericsあり
        // 各パラメータをu64にキャストし、黄金比定数で掛け合わせながら加算する
        // これにより<N1, N2>と<N2, N1>は異なるextra値になる
        let param_idents: Vec<_> = const_params.iter().map(|p| &p.ident).collect();
        quote! {
            {
                let extra: u64 = 0u64
                    #(  .wrapping_add(#param_idents as u64)
                        .wrapping_mul(0x9e3779b97f4a7c15u64) )*;
                BitSet {
                    bits: [
                        fnv64_seed_with_int(#name_str, extra, SEEDS[0]) #eb0,
                        fnv64_seed_with_int(#name_str, extra, SEEDS[1]) #eb1,
                        fnv64_seed_with_int(#name_str, extra, SEEDS[2]) #eb2,
                        fnv64_seed_with_int(#name_str, extra, SEEDS[3]) #eb3,
                    ]
                }
            }
        }
    };

    quote! {
        impl #impl_generics Pred for #name #ty_generics #where_clause {
            const PRED_BIT: BitSet = #bit_expr;
        }
    }
    .into()
}
