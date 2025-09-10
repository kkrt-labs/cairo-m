#![allow(non_snake_case)]

//! Component for Poseidon2 hash function permutation over M31 field.
//! Implements the complete Poseidon2 permutation with external and internal rounds.
//!
//! # Columns
//!
//! For each (a >>> n1) ^ (a >>> n2) ^ (a >>> n3) operation (with n1 <= n2 <= n3):
//! We define the following bit indices sets:
//!  * input_0 = { ( ( n2 - n1 ) * a + ( n3 - n1 ) * b ) mod 32 ; 0 <= a, b < 4}
//!  * input_1 = [0, 31] \ input_0
//! Use half of input_0's bits for a0, the other half for a1.
//! Use half of input_1's bits for a2, the other half for a3.
//!
//! * W ([u32; 64]):
//!   * for t in 0..16 : 4 columns per W[t]
//!   * for t in 16..61 : 4 columns per W[t]
//!
//! # Constraints
//!
//! * enabler is a bool
//!   * `enabler * (1 - enabler)`use std::ops::{Add, AddAssign, Mul, Sub};
//! * lookup relations:
//!   * `- enabler * [initial_state]` in `Poseidon2` relation

use std::ops::{Add, AddAssign, Mul, Sub};

use num_traits::{One, Zero};
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval, RelationEntry};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::FieldExpOps;

use crate::components::sha256::witness::Claim;
use crate::components::Relations;

const INV16: M31 = M31::from_u32_unchecked(1 << 15);
const TWO: M31 = M31::from_u32_unchecked(2);
const N_LOG_INSTANCES_PER_ROW: usize = 3;
const N_INSTANCES_PER_ROW: usize = 1 << N_LOG_INSTANCES_PER_ROW;
const N_STATE: usize = 16;
const N_PARTIAL_ROUNDS: usize = 14;
const N_HALF_FULL_ROUNDS: usize = 4;
const FULL_ROUNDS: usize = 2 * N_HALF_FULL_ROUNDS;
const N_COLUMNS_PER_REP: usize = N_STATE * (1 + FULL_ROUNDS) + N_PARTIAL_ROUNDS;
const N_COLUMNS: usize = N_INSTANCES_PER_ROW * N_COLUMNS_PER_REP;
const LOG_EXPAND: u32 = 2;

enum SigmaType {
    SmallSigma0,
    SmallSigma1,
    BigSigma0,
    BigSigma1,
}

/// Utility for representing a u32 as two field elements, for constraint evaluation.
#[derive(Clone, Debug)]
struct Fu32<F>
where
    F: FieldExpOps
        + Clone
        + AddAssign<F>
        + Add<F, Output = F>
        + Sub<F, Output = F>
        + Mul<M31, Output = F>,
{
    lo: F,
    hi: F,
}
impl<F> Fu32<F>
where
    F: FieldExpOps
        + Clone
        + AddAssign<F>
        + Add<F, Output = F>
        + Sub<F, Output = F>
        + Mul<M31, Output = F>,
{
    fn into_felts(self) -> [F; 2] {
        [self.lo, self.hi]
    }
}

pub type Sha256Component = FrameworkComponent<Sha256Eval>;

#[derive(Clone)]
pub struct Sha256Eval {
    pub claim: Claim,
    pub relations: Relations,
}
impl FrameworkEval for Sha256Eval {
    fn log_size(&self) -> u32 {
        self.claim.log_size
    }
    fn max_constraint_log_degree_bound(&self) -> u32 {
        self.claim.log_size + LOG_EXPAND
    }
    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        let zero = E::F::from(M31::zero());
        let one = E::F::from(M31::one());
        let K: [Fu32<E::F>; 64] = std::array::from_fn(|_| Fu32 {
            lo: zero.clone(),
            hi: zero.clone(),
        });
        let mut H: [Fu32<E::F>; 8] = std::array::from_fn(|_| Fu32 {
            lo: zero.clone(),
            hi: zero.clone(),
        });

        // ╔════════════════════════════════════╗
        // ║             Scheduling             ║
        // ╚════════════════════════════════════╝
        let mut W: [Fu32<E::F>; 64] = std::array::from_fn(|_| Fu32 {
            lo: zero.clone(),
            hi: zero.clone(),
        });

        // Load message
        (0..16).for_each(|i| {
            // Load lo and hi bits
            W[i].lo = eval.next_trace_mask();
            W[i].hi = eval.next_trace_mask();
            eval.add_to_relation(RelationEntry::new(
                &self.relations.range_check_16,
                E::EF::from(one.clone()),
                &[W[i].lo.clone()],
            ));
            eval.add_to_relation(RelationEntry::new(
                &self.relations.range_check_16,
                E::EF::from(one.clone()),
                &[W[i].hi.clone()],
            ));
        });

        // Compute message schedule
        for i in 16..64 {
            // TODO: W[i-15] and W[i-2] are not in temp sum so they could be decomposed in 4 limbs instead of 6
            let s0 = self.sigma(&mut eval, SigmaType::SmallSigma0, None);
            let s1 = self.sigma(&mut eval, SigmaType::SmallSigma1, None);
            let temp = add3_u32_unchecked(&mut eval, W[i - 16].clone(), W[i - 7].clone(), s0);
            W[i] = add2_u32_unchecked(&mut eval, temp, s1);
        }

        // ╔════════════════════════════════════╗
        // ║             Rounds                 ║
        // ╚════════════════════════════════════╝
        for i in 0..64 {
            let a: [E::F; 6] = std::array::from_fn(|_| eval.next_trace_mask());
            let b: [E::F; 6] = std::array::from_fn(|_| eval.next_trace_mask());
            let c: [E::F; 6] = std::array::from_fn(|_| eval.next_trace_mask());
            let d: Fu32<E::F> = Fu32 {
                lo: eval.next_trace_mask(),
                hi: eval.next_trace_mask(),
            };
            let e: [E::F; 6] = std::array::from_fn(|_| eval.next_trace_mask());
            let f: [E::F; 6] = std::array::from_fn(|_| eval.next_trace_mask());
            let g: [E::F; 6] = std::array::from_fn(|_| eval.next_trace_mask());
            let h: Fu32<E::F> = Fu32 {
                lo: eval.next_trace_mask(),
                hi: eval.next_trace_mask(),
            };

            let S0 = self.sigma(&mut eval, SigmaType::BigSigma0, Some(a.clone()));
            let S1 = self.sigma(&mut eval, SigmaType::BigSigma1, Some(e.clone()));
            let ch = self.ch(&mut eval, e.clone(), f.clone(), g.clone());
            let maj = self.maj(&mut eval, a.clone(), b.clone(), c.clone());
            let temp0 = add3_u32_unchecked(&mut eval, h, ch, S1);
            let temp1 = add3_u32_unchecked(&mut eval, temp0, K[i].clone(), W[i].clone());
            let temp2 = add2_u32_unchecked(&mut eval, S0, maj);

            H[0] = add3_u32_unchecked(&mut eval, temp1.clone(), temp2, H[0].clone());
            H[1] = add2_u32_unchecked(
                &mut eval,
                H[1].clone(),
                Fu32 {
                    lo: a[0].clone() + a[1].clone() + a[2].clone(),
                    hi: a[3].clone() + a[4].clone() + a[5].clone(),
                },
            );
            H[2] = add2_u32_unchecked(
                &mut eval,
                H[2].clone(),
                Fu32 {
                    lo: b[0].clone() + b[1].clone() + b[2].clone(),
                    hi: b[3].clone() + b[4].clone() + b[5].clone(),
                },
            );
            H[3] = add2_u32_unchecked(
                &mut eval,
                H[3].clone(),
                Fu32 {
                    lo: c[0].clone() + c[1].clone() + c[2].clone(),
                    hi: c[3].clone() + c[4].clone() + c[5].clone(),
                },
            );
            H[4] = add3_u32_unchecked(&mut eval, d, temp1, H[4].clone());
            H[5] = add2_u32_unchecked(
                &mut eval,
                H[5].clone(),
                Fu32 {
                    lo: e[0].clone() + e[1].clone() + e[2].clone(),
                    hi: e[3].clone() + e[4].clone() + e[5].clone(),
                },
            );
            H[6] = add2_u32_unchecked(
                &mut eval,
                H[6].clone(),
                Fu32 {
                    lo: f[0].clone() + f[1].clone() + f[2].clone(),
                    hi: f[3].clone() + f[4].clone() + f[5].clone(),
                },
            );
            H[7] = add2_u32_unchecked(
                &mut eval,
                H[7].clone(),
                Fu32 {
                    lo: g[0].clone() + g[1].clone() + g[2].clone(),
                    hi: g[3].clone() + g[4].clone() + g[5].clone(),
                },
            );
        }
        eval.finalize_logup_in_pairs();

        eval
    }
}

impl Sha256Eval {
    fn sigma<E: EvalAtRow>(
        &self,
        eval: &mut E,
        sigma: SigmaType,
        provided_limbs: Option<[E::F; 6]>,
    ) -> Fu32<E::F> {
        let (l0, l1, l2, h0, h1, h2) = if let Some(limbs) = provided_limbs {
            (
                limbs[0].clone(),
                limbs[1].clone(),
                limbs[2].clone(),
                limbs[3].clone(),
                limbs[4].clone(),
                limbs[5].clone(),
            )
        } else {
            (
                eval.next_trace_mask(),
                eval.next_trace_mask(),
                eval.next_trace_mask(),
                eval.next_trace_mask(),
                eval.next_trace_mask(),
                eval.next_trace_mask(),
            )
        };

        let out0_lo = eval.next_trace_mask();
        let out0_hi = eval.next_trace_mask();
        let out1_lo = eval.next_trace_mask();
        let out1_hi = eval.next_trace_mask();
        let out2_0 = eval.next_trace_mask();
        let out2_1 = eval.next_trace_mask();
        let mut out2_2 = None;
        let mut out2_3 = None;

        if let SigmaType::BigSigma0 = sigma {
            out2_2 = Some(eval.next_trace_mask());
            out2_3 = Some(eval.next_trace_mask());
        }

        let xor_out2_lo = eval.next_trace_mask();
        let xor_out2_hi = eval.next_trace_mask();

        // Compute output of small sigma0 for first set of bits
        // BigSigma0 has special treatment because the set of bits that are affected by
        // both L0||H0||H1 and L1||L2||H2 is too large (we would need to XOR two words of 15 bits)
        eval.add_to_relation(RelationEntry::new(
            match sigma {
                SigmaType::SmallSigma0 => &self.relations.small_sigma0,
                SigmaType::SmallSigma1 => &self.relations.small_sigma1,
                SigmaType::BigSigma0 => &self.relations.big_sigma0,
                SigmaType::BigSigma1 => &self.relations.big_sigma1,
            },
            E::EF::from(one.clone()),
            match sigma {
                SigmaType::BigSigma0 => &[l0, h0, h1, out0_lo, out0_hi, out2_0, out2_2.unwrap()],
                _ => &[l0, h0, h1, out0_lo, out0_hi, out2_0],
            },
        ));
        // Compute output of small sigma0 for second set of bits
        eval.add_to_relation(RelationEntry::new(
            match sigma {
                SigmaType::SmallSigma0 => &self.relations.small_sigma0,
                SigmaType::SmallSigma1 => &self.relations.small_sigma1,
                SigmaType::BigSigma0 => &self.relations.big_sigma0,
                SigmaType::BigSigma1 => &self.relations.big_sigma1,
            },
            E::EF::from(one.clone()),
            match sigma {
                SigmaType::BigSigma0 => &[l1, l2, h2, out1_lo, out1_hi, out2_1, out2_3.unwrap()],
                _ => &[l1, l2, h2, out1_lo, out1_hi, out2_1],
            },
        ));

        // Finalize computation of third set of bits
        eval.add_to_relation(RelationEntry::new(
            match sigma {
                SigmaType::SmallSigma0 => &self.relations.xor_small_sigma0,
                SigmaType::SmallSigma1 => &self.relations.xor_small_sigma1,
                SigmaType::BigSigma0 => &self.relations.xor_big_sigma0,
                SigmaType::BigSigma1 => &self.relations.xor_big_sigma1,
            },
            E::EF::from(one.clone()),
            match sigma {
                SigmaType::BigSigma0 => &[out2_0, out2_1, xor_out2_lo], //xor_out2_lo is a function of out2_0 and out2_1 only
                _ => &[out2_0, out2_1, xor_out2_lo, xor_out2_hi],
            },
        ));

        if let SigmaType::BigSigma0 = sigma {
            eval.add_to_relation(RelationEntry::new(
                &self.relations.xor_big_sigma0,
                E::EF::from(one.clone()),
                &[out2_2.unwrap(), out2_3.unwrap(), xor_out2_hi], //xor_out2_hi is a function of out2_2 and out2_3 only
            ));
        }

        // Add all limbs together to rebuild the 32-bit result
        let out0 = Fu32 {
            lo: out0_lo,
            hi: out0_hi,
        };
        let out1 = Fu32 {
            lo: out1_lo,
            hi: out1_hi,
        };
        let out2 = Fu32 {
            lo: xor_out2_lo,
            hi: xor_out2_hi,
        };
        let res = add3_u32_unchecked(eval, out0, out1, out2);

        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            E::EF::from(one.clone()),
            &[res.lo],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            E::EF::from(one.clone()),
            &[res.hi],
        ));

        res
    }

    fn ch<E: EvalAtRow>(
        &self,
        eval: &mut E,
        e: [E::F; 6],
        f: [E::F; 6],
        g: [E::F; 6],
    ) -> Fu32<E::F> {
        let ch = std::array::from_fn(|_| eval.next_trace_mask());

        eval.add_to_relation(RelationEntry::new(
            &self.relations.ch,
            E::EF::from(one.clone()),
            &[e[0], f[0], g[0], ch[0]],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.ch,
            E::EF::from(one.clone()),
            &[e[1], f[1], g[1], ch[1]],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.ch,
            E::EF::from(one.clone()),
            &[e[2], f[2], g[2], ch[2]],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.ch,
            E::EF::from(one.clone()),
            &[e[3], f[3], g[3], ch[3]],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.ch,
            E::EF::from(one.clone()),
            &[e[4], f[4], g[4], ch[4]],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.ch,
            E::EF::from(one.clone()),
            &[e[5], f[5], g[5], ch[5]],
        ));

        Fu32 {
            lo: ch[0] + ch[1] + ch[2],
            hi: ch[3] + ch[4] + ch[5],
        }
    }

    fn maj<E: EvalAtRow>(
        &self,
        eval: &mut E,
        a: [E::F; 6],
        b: [E::F; 6],
        c: [E::F; 6],
    ) -> Fu32<E::F> {
        let maj = std::array::from_fn(|_| eval.next_trace_mask());

        eval.add_to_relation(RelationEntry::new(
            &self.relations.maj,
            E::EF::from(one.clone()),
            &[a[0], b[0], c[0], maj[0]],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.maj,
            E::EF::from(one.clone()),
            &[a[1], b[1], c[1], maj[1]],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.maj,
            E::EF::from(one.clone()),
            &[a[2], b[2], c[2], maj[2]],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.maj,
            E::EF::from(one.clone()),
            &[a[3], b[3], c[3], maj[3]],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.maj,
            E::EF::from(one.clone()),
            &[a[4], b[4], c[4], maj[4]],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.maj,
            E::EF::from(one.clone()),
            &[a[5], b[5], c[5], maj[5]],
        ));

        Fu32 {
            lo: maj[0] + maj[1] + maj[2],
            hi: maj[3] + maj[4] + maj[5],
        }
    }
}

/// Adds two u32s, returning the sum.
/// Assumes a, b are properly range checked.
/// The caller is responsible for checking:
/// res.{l,h} not in [2^16, 2^17) or in [-2^16,0)
fn add2_u32_unchecked<E: EvalAtRow>(eval: &mut E, a: Fu32<E::F>, b: Fu32<E::F>) -> Fu32<E::F> {
    let sl = eval.next_trace_mask();
    let sh = eval.next_trace_mask();

    let carry_l = (a.lo + b.lo - sl.clone()) * E::F::from(INV16);
    eval.add_constraint(carry_l.clone() * carry_l.clone() - carry_l.clone());

    let carry_h = (a.hi + b.hi + carry_l - sh.clone()) * E::F::from(INV16);
    eval.add_constraint(carry_h.clone() * carry_h.clone() - carry_h.clone());

    Fu32 { lo: sl, hi: sh }
}

/// Adds three u32s, returning the sum.
/// Assumes a, b, c are properly range checked.
/// Caller is responsible for checking:
/// res.{l,h} not in [2^16, 3*2^16) or in [-2^17,0)
fn add3_u32_unchecked<E: EvalAtRow>(
    eval: &mut E,
    a: Fu32<E::F>,
    b: Fu32<E::F>,
    c: Fu32<E::F>,
) -> Fu32<E::F> {
    let sl = eval.next_trace_mask();
    let sh = eval.next_trace_mask();

    let carry_l = (a.lo + b.lo + c.lo - sl.clone()) * E::F::from(INV16);
    eval.add_constraint(
        carry_l.clone() * (carry_l.clone() - E::F::one()) * (carry_l.clone() - E::F::from(TWO)),
    );

    let carry_h = (a.hi + b.hi + c.hi + carry_l - sh.clone()) * E::F::from(INV16);
    eval.add_constraint(
        carry_h.clone() * (carry_h.clone() - E::F::one()) * (carry_h.clone() - E::F::from(TWO)),
    );

    Fu32 {
        lo: sl,
        hi: sh.clone(),
    }
}
