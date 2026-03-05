use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

/// 制約型に[`Proof`]を自動実装するderiveマクロ。
///
/// 型の名前とconstジェネリクスパラメータから`BitSet`を定数計算で生成し、
/// `Proof::PROOF_BIT`として実装する。
///
/// ## ビットセットの生成方法
/// ### constジェネリクスを持たない型
/// 型名の文字列を4つの異なるシード値でFNV-64ハッシュし、その結果を`bits`に並べる。
/// ### constジェネリクスを持つ型
/// 複数のconstパラメータをリトルエンディアン的に合算した`extra`を計算し、型名と`extra`を混ぜてハッシュする。<br>
/// よって、`<N1, N2>` と`<N2, N1>`は別のビットセットとなる。
/// ## ⚠️ 誤判定の可能性
/// パラメータ値を`extra: u64`という単一の変数に畳み込むため、衝突が起きる可能性がある。
#[proc_macro_derive(Proof)]
pub fn derive_validator_proof(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let name = &input.ident;
    let name_str = name.to_string();
    let const_params: Vec<_> = input.generics.const_params().collect();

    let bit_expr = if const_params.is_empty() {
        // 通常の型
        quote! {
            BitSet {
                bits: [
                    fnv64_seed(#name_str, SEEDS[0]),
                    fnv64_seed(#name_str, SEEDS[1]),
                    fnv64_seed(#name_str, SEEDS[2]),
                    fnv64_seed(#name_str, SEEDS[3]),
                ]
            }
        }
    } else {
        // const generics: 型名 + 各パラメータの値を混ぜる
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
                        fnv64_seed_with_int(#name_str, extra, SEEDS[0]),
                        fnv64_seed_with_int(#name_str, extra, SEEDS[1]),
                        fnv64_seed_with_int(#name_str, extra, SEEDS[2]),
                        fnv64_seed_with_int(#name_str, extra, SEEDS[3]),
                    ]
                }
            }
        }
    };

    quote! {
        impl #impl_generics Proof for #name #ty_generics #where_clause {
            const PROOF_BIT: BitSet = #bit_expr;
        }
    }
    .into()
}
