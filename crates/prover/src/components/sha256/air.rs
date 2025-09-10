#![allow(non_snake_case)]

use num_traits::{One, Zero};
use stwo_constraint_framework::{EvalAtRow, FrameworkEval, RelationEntry};
use stwo_prover::core::fields::m31::M31;

use crate::components::sha256::{Eval, Fu32, SigmaType};

const INV16: M31 = M31::from_u32_unchecked(1 << 15);
const TWO: M31 = M31::from_u32_unchecked(2);

impl FrameworkEval for Eval {
    fn log_size(&self) -> u32 {
        self.claim.log_size
    }
    fn max_constraint_log_degree_bound(&self) -> u32 {
        self.claim.log_size + 1
    }
    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        let one = E::F::from(M31::one());
        let K: [Fu32<E::F>; 64] = std::array::from_fn(|_| Fu32::zero());
        let mut H: [Fu32<E::F>; 8] = std::array::from_fn(|_| Fu32::zero());

        // ╔════════════════════════════════════╗
        // ║             Scheduling             ║
        // ╚════════════════════════════════════╝
        let mut W: [Fu32<E::F>; 64] = std::array::from_fn(|_| Fu32::zero());

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
            let w_i_minus_15 = std::array::from_fn(|_| eval.next_trace_mask());
            let s0 = self.sigma(SigmaType::SmallSigma0, w_i_minus_15, &mut eval);

            let w_i_minus_2 = std::array::from_fn(|_| eval.next_trace_mask());
            let s1 = self.sigma(SigmaType::SmallSigma1, w_i_minus_2, &mut eval);

            let temp = add3_u32_unchecked(W[i - 16].clone(), W[i - 7].clone(), s0, &mut eval);
            W[i] = add2_u32_unchecked(temp, s1, &mut eval);
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

            let S0 = self.sigma(SigmaType::BigSigma0, a.clone(), &mut eval);
            let S1 = self.sigma(SigmaType::BigSigma1, e.clone(), &mut eval);
            let ch = self.ch(e.clone(), f.clone(), g.clone(), &mut eval);
            let maj = self.maj(a.clone(), b.clone(), c.clone(), &mut eval);
            let temp0 = add3_u32_unchecked(h, ch, S1, &mut eval);
            let temp1 = add3_u32_unchecked(temp0, K[i].clone(), W[i].clone(), &mut eval);
            let temp2 = add2_u32_unchecked(S0, maj, &mut eval);

            H[0] = add3_u32_unchecked(temp1.clone(), temp2, H[0].clone(), &mut eval);
            H[1] = add2_u32_unchecked(
                H[1].clone(),
                Fu32 {
                    lo: a[0].clone() + a[1].clone() + a[2].clone(),
                    hi: a[3].clone() + a[4].clone() + a[5].clone(),
                },
                &mut eval,
            );
            H[2] = add2_u32_unchecked(
                H[2].clone(),
                Fu32 {
                    lo: b[0].clone() + b[1].clone() + b[2].clone(),
                    hi: b[3].clone() + b[4].clone() + b[5].clone(),
                },
                &mut eval,
            );
            H[3] = add2_u32_unchecked(
                H[3].clone(),
                Fu32 {
                    lo: c[0].clone() + c[1].clone() + c[2].clone(),
                    hi: c[3].clone() + c[4].clone() + c[5].clone(),
                },
                &mut eval,
            );
            H[4] = add3_u32_unchecked(d, temp1, H[4].clone(), &mut eval);
            H[5] = add2_u32_unchecked(
                H[5].clone(),
                Fu32 {
                    lo: e[0].clone() + e[1].clone() + e[2].clone(),
                    hi: e[3].clone() + e[4].clone() + e[5].clone(),
                },
                &mut eval,
            );
            H[6] = add2_u32_unchecked(
                H[6].clone(),
                Fu32 {
                    lo: f[0].clone() + f[1].clone() + f[2].clone(),
                    hi: f[3].clone() + f[4].clone() + f[5].clone(),
                },
                &mut eval,
            );
            H[7] = add2_u32_unchecked(
                H[7].clone(),
                Fu32 {
                    lo: g[0].clone() + g[1].clone() + g[2].clone(),
                    hi: g[3].clone() + g[4].clone() + g[5].clone(),
                },
                &mut eval,
            );
        }
        eval.finalize_logup_in_pairs();

        eval
    }
}

impl Eval {
    fn sigma<E: EvalAtRow>(
        &self,
        sigma: SigmaType,
        [l0, l1, l2, h0, h1, h2]: [E::F; 6],
        eval: &mut E,
    ) -> Fu32<E::F> {
        let one = E::F::one();
        let [out0_lo, out0_hi, out1_lo, out1_hi, out2_0, out2_1] =
            std::array::from_fn(|_| eval.next_trace_mask());

        let (out2_0_lo, out2_1_lo, out2_0_hi, out2_1_hi) = match sigma {
            SigmaType::BigSigma0 => (
                Some(out2_0.clone()),
                Some(out2_1.clone()),
                Some(eval.next_trace_mask()),
                Some(eval.next_trace_mask()),
            ),
            _ => (Some(out2_0.clone()), Some(out2_1.clone()), None, None),
        };

        let xor_out2_lo = eval.next_trace_mask();
        let xor_out2_hi = eval.next_trace_mask();

        // Compute output of small sigma0 for first set of bits
        // BigSigma0 has special treatment because the set of bits that are affected by
        // both L0||H0||H1 and L1||L2||H2 is too large (we would need to XOR two words of 15 bits)
        match sigma {
            SigmaType::SmallSigma0 => {
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.small_sigma0_0,
                    E::EF::from(one.clone()),
                    &[l1, l2, h2, out0_lo.clone(), out0_hi.clone(), out2_0.clone()],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.small_sigma0_1,
                    E::EF::from(one.clone()),
                    &[l0, h0, h1, out1_lo.clone(), out1_hi.clone(), out2_1.clone()],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.xor_small_sigma0,
                    E::EF::from(one.clone()),
                    &[
                        out2_0.clone(),
                        out2_1.clone(),
                        xor_out2_lo.clone(),
                        xor_out2_hi.clone(),
                    ],
                ));
            }
            SigmaType::SmallSigma1 => {
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.small_sigma1_0,
                    E::EF::from(one.clone()),
                    &[l0, h0, out0_lo.clone(), out0_hi.clone(), out2_0.clone()],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.small_sigma1_1,
                    E::EF::from(one.clone()),
                    &[
                        l1,
                        l2,
                        h1,
                        h2,
                        out1_lo.clone(),
                        out1_hi.clone(),
                        out2_1.clone(),
                    ],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.xor_small_sigma1,
                    E::EF::from(one.clone()),
                    &[
                        out2_0.clone(),
                        out2_1.clone(),
                        xor_out2_lo.clone(),
                        xor_out2_hi.clone(),
                    ],
                ));
            }
            SigmaType::BigSigma0 => {
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.big_sigma0_0,
                    E::EF::from(one.clone()),
                    &[
                        l1,
                        l2,
                        h2,
                        out0_lo.clone(),
                        out0_hi.clone(),
                        out2_0_lo.clone().unwrap(),
                        out2_0_hi.clone().unwrap(),
                    ],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.big_sigma0_1,
                    E::EF::from(one.clone()),
                    &[
                        l0,
                        h0,
                        h1,
                        out1_lo.clone(),
                        out1_hi.clone(),
                        out2_1_lo.clone().unwrap(),
                        out2_1_hi.clone().unwrap(),
                    ],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.xor_big_sigma0_0,
                    E::EF::from(one.clone()),
                    &[
                        out2_0_lo.clone().unwrap(),
                        out2_1_lo.clone().unwrap(),
                        xor_out2_lo.clone(),
                    ],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.xor_big_sigma0_1,
                    E::EF::from(one.clone()),
                    &[
                        out2_0_hi.clone().unwrap(),
                        out2_1_hi.clone().unwrap(),
                        xor_out2_hi.clone(),
                    ],
                ));
            }
            SigmaType::BigSigma1 => {
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.big_sigma1_0,
                    E::EF::from(one.clone()),
                    &[l0, h0, h1, out0_lo.clone(), out0_hi.clone(), out2_0.clone()],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.big_sigma1_1,
                    E::EF::from(one.clone()),
                    &[l1, l2, h2, out1_lo.clone(), out1_hi.clone(), out2_1.clone()],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.xor_big_sigma1,
                    E::EF::from(one.clone()),
                    &[
                        out2_0.clone(),
                        out2_1.clone(),
                        xor_out2_lo.clone(),
                        xor_out2_hi.clone(),
                    ],
                ));
            }
        };

        // Add all limbs together to rebuild the 32-bit result
        let out0 = Fu32 {
            lo: out0_lo.clone(),
            hi: out0_hi.clone(),
        };
        let out1 = Fu32 {
            lo: out1_lo.clone(),
            hi: out1_hi.clone(),
        };
        let out2 = Fu32 {
            lo: xor_out2_lo.clone(),
            hi: xor_out2_hi.clone(),
        };
        let res = add3_u32_unchecked(out0, out1, out2, eval);

        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            E::EF::from(one.clone()),
            &[res.lo.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            E::EF::from(one.clone()),
            &[res.hi.clone()],
        ));

        res
    }

    fn ch<E: EvalAtRow>(
        &self,
        e: [E::F; 6],
        f: [E::F; 6],
        g: [E::F; 6],
        eval: &mut E,
    ) -> Fu32<E::F> {
        let ch = std::array::from_fn(|_| eval.next_trace_mask());

        (0..6).for_each(|i| {
            eval.add_to_relation(RelationEntry::new(
                &self.relations.ch,
                E::EF::from(one.clone()),
                &[e[i], f[i], g[i], ch[i]],
            ));
        });

        Fu32 {
            lo: ch[0] + ch[1] + ch[2],
            hi: ch[3] + ch[4] + ch[5],
        }
    }

    fn maj<E: EvalAtRow>(
        &self,
        a: [E::F; 6],
        b: [E::F; 6],
        c: [E::F; 6],
        eval: &mut E,
    ) -> Fu32<E::F> {
        let maj = std::array::from_fn(|_| eval.next_trace_mask());

        (0..6).for_each(|i| {
            eval.add_to_relation(RelationEntry::new(
                &self.relations.maj,
                E::EF::from(one.clone()),
                &[a[i], b[i], c[i], maj[i]],
            ));
        });

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
fn add2_u32_unchecked<E: EvalAtRow>(a: Fu32<E::F>, b: Fu32<E::F>, eval: &mut E) -> Fu32<E::F> {
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
    a: Fu32<E::F>,
    b: Fu32<E::F>,
    c: Fu32<E::F>,
    eval: &mut E,
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
