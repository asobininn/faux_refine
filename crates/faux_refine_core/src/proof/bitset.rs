use crate::proof::list::{Cons, Nil};

/// ビットセットの幅(256bit)
const BITS_NUM: usize = 4;

/// FNV-64ハッシュで使用する4つのシード値(xxHashより引用)
pub const SEEDS: [u64; BITS_NUM] = [
    0x9e3779b185ebca87,
    0xc2b2ae3d27d4eb4f,
    0x165667b19e3779f9,
    0x85ebca77c2b2ae63,
];

#[derive(Debug, Clone, Copy)]
pub struct BitSet {
    pub bits: [u64; 4],
}

impl BitSet {
    /// selfがtargetの部分集合かを判定する。
    /// ### ⚠️ハッシュ衝突による誤判定あり
    /// 包含関係がないのに`true`を返す可能性あり。
    pub const fn is_subset_of(&self, target: &Self) -> bool {
        let mut i = 0;
        while i < BITS_NUM {
            if (self.bits[i] & !target.bits[i]) != 0 {
                return false;
            }
            i += 1;
        }
        true
    }
}

pub trait Proof {
    const PROOF_BIT: BitSet;
}

impl Proof for Nil {
    const PROOF_BIT: BitSet = BitSet { bits: [0, 0, 0, 0] };
}

impl<V: Proof, Rest: Proof> Proof for Cons<V, Rest> {
    const PROOF_BIT: BitSet = BitSet {
        bits: [
            V::PROOF_BIT.bits[0] | Rest::PROOF_BIT.bits[0],
            V::PROOF_BIT.bits[1] | Rest::PROOF_BIT.bits[1],
            V::PROOF_BIT.bits[2] | Rest::PROOF_BIT.bits[2],
            V::PROOF_BIT.bits[3] | Rest::PROOF_BIT.bits[3],
        ],
    };
}

pub const fn fnv64_seed(s: &str, seed: u64) -> u64 {
    // Fowler–Noll–Vo hash function
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001b3;
    let mut hash = FNV_OFFSET;
    let bytes = s.as_bytes();
    hash ^= seed;
    hash = hash.wrapping_mul(FNV_PRIME);
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    hash
}

pub const fn fnv64_seed_with_int(s: &str, n: u64, seed: u64) -> u64 {
    // Fowler–Noll–Vo hash function
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001b3;
    let mut hash = FNV_OFFSET;
    // 最初に文字列をハッシュ
    let bytes = s.as_bytes();
    hash ^= seed;
    hash = hash.wrapping_mul(FNV_PRIME);
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    // 次にintを8バイトとしてハッシュ
    let mut shift = 0;
    while shift < 64 {
        hash ^= ((n >> shift) & 0xff) as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        shift += 8;
    }
    hash
}
