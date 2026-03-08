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

/// A derive macro that automatically implements [Pred] for constraint types.
///
/// It generates a BitSet at compile time from the type name and its const generic parameters,
/// implements it as `Pred::PRED_BIT`.
/// ## Examples
/// ```rust
/// #[derive(Pred, Debug, Clone)]
/// struct Square;
///
/// #[derive(Pred, Debug, Clone)]
/// #[pred(extends(Square))]
/// struct NonSingular;
///
/// #[derive(Pred, Debug, Clone)]
/// #[pred(extends(NonSingular))]
/// struct PositiveDefinite;
/// ```
/// ## Constraint Inheritance with `extends`
/// When `#[Pred(extends(..))]` is specified, the parent constraint’s `PRED_BIT` values are combined using a bitwise OR.
///
/// ## ⚠️ About Potential Probabilistic Bugs
/// This crate may experience two types of collisions:
///
/// 1. FNV-64 hash collisions (affects all constraint types)
/// 2. Collisions of the extra value (affects types with const generics)
///
/// Both types of collisions can only result in false positives (i.e., succeeding when they should fail).
#[proc_macro_derive(Pred, attributes(pred))]
pub fn derive_validator_pred(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let name = &input.ident;
    let name_str = name.to_string();
    let const_params: Vec<_> = input.generics.const_params().collect();

    // #[pred(extends(..))]をパースする
    let extends: Vec<Type> = {
        let mut result = Vec::new();
        for attr in input.attrs.iter().filter(|a| a.path().is_ident("pred")) {
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
            {
                // const MANGLE: &str =
                const MANGLED: &str = concat!(module_path!(), "::", #name_str);
                BitSet {
                    bits: [
                        fnv64_seed(MANGLED, SEEDS[0]) #eb0,
                        fnv64_seed(MANGLED, SEEDS[1]) #eb1,
                        fnv64_seed(MANGLED, SEEDS[2]) #eb2,
                        fnv64_seed(MANGLED, SEEDS[3]) #eb3,
                    ]
                }
            }
        }
    } else {
        // const genericsあり
        // 各パラメータをu64にキャストし、黄金比定数で掛け合わせながら加算する
        // これにより<N1, N2>と<N2, N1>は異なるextra値になる
        let param_idents: Vec<_> = const_params.iter().map(|p| &p.ident).collect();
        quote! {
            {
                const MANGLED: &str = concat!(module_path!(), "::", #name_str);
                let extra: u64 = 0u64
                    #(  .wrapping_add(#param_idents as u64)
                        .wrapping_mul(0x9e3779b97f4a7c15u64) )*;
                BitSet {
                    bits: [
                        fnv64_seed_with_int(MANGLED, extra, SEEDS[0]) #eb0,
                        fnv64_seed_with_int(MANGLED, extra, SEEDS[1]) #eb1,
                        fnv64_seed_with_int(MANGLED, extra, SEEDS[2]) #eb2,
                        fnv64_seed_with_int(MANGLED, extra, SEEDS[3]) #eb3,
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
