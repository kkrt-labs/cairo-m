#![allow(non_snake_case)]

use num_traits::{One, Zero};
use stwo_constraint_framework::{EvalAtRow, FrameworkEval, RelationEntry};
use stwo_prover::core::fields::m31::M31;

use crate::components::sha256::{Eval, Fu32_2, Fu32_4, SigmaType};

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
        let one = E::EF::one();
        // Allocate large arrays on heap to avoid stack overflow
        let K: Box<[Fu32_2<E::F>; 64]> = Box::new(std::array::from_fn(|_| Fu32_2::zero()));
        let mut H: Box<[Fu32_4<E::F>; 8]> = Box::new(std::array::from_fn(|_| Fu32_4::zero()));

        // ╔════════════════════════════════════╗
        // ║             Scheduling             ║
        // ╚════════════════════════════════════╝
        let mut W: Box<[Fu32_2<E::F>; 64]> = Box::new(std::array::from_fn(|_| Fu32_2::zero()));

        // Load message
        (0..16).for_each(|i| {
            // Load lo and hi bits
            W[i].lo = eval.next_trace_mask();
            W[i].hi = eval.next_trace_mask();
            eval.add_to_relation(RelationEntry::new(
                &self.relations.range_check_16,
                -one.clone(),
                &[W[i].lo.clone()],
            ));
            eval.add_to_relation(RelationEntry::new(
                &self.relations.range_check_16,
                -one.clone(),
                &[W[i].hi.clone()],
            ));
        }); // 2304 + 2 * 16 = 2336

        // Compute message schedule
        for i in 16..64 {
            // TODO: W[i-15] and W[i-2] are not in temp sum so they could be decomposed in 4 limbs instead of 6
            let w_i_minus_15 = std::array::from_fn(|_| eval.next_trace_mask());
            let s0 = self.sigma(SigmaType::SmallSigma0, w_i_minus_15, &mut eval);

            let w_i_minus_2 = std::array::from_fn(|_| eval.next_trace_mask());
            let s1 = self.sigma(SigmaType::SmallSigma1, w_i_minus_2, &mut eval);

            let temp = add3_u32_unchecked(W[i - 16].clone(), W[i - 7].clone(), s0, &mut eval);
            W[i] = add2_u32_unchecked(temp, s1, &mut eval);
        } // 2576

        // ╔════════════════════════════════════╗
        // ║             Rounds                 ║
        // ╚════════════════════════════════════╝
        for i in 0..64 {
            let a: [E::F; 6] = std::array::from_fn(|_| eval.next_trace_mask());
            let b = H[1].clone();
            let c = H[2].clone();
            let d = H[3].clone();
            let e: [E::F; 6] = std::array::from_fn(|_| eval.next_trace_mask());
            let f = H[5].clone();
            let g = H[6].clone();
            let h = H[7].clone();

            let S0 = self.sigma(SigmaType::BigSigma0, a.clone(), &mut eval);
            let S1 = self.sigma(SigmaType::BigSigma1, e.clone(), &mut eval);
            let (e_4, ch) = self.ch(e.clone(), f.clone(), g.clone(), &mut eval);
            let (a_4, maj) = self.maj(a.clone(), b.clone(), c.clone(), &mut eval);
            let temp1 = add3_u32_unchecked(
                add3_u32_unchecked(h.clone().into(), ch, S1, &mut eval),
                K[i].clone(),
                W[i].clone(),
                &mut eval,
            );
            let temp2 = add2_u32_unchecked(S0, maj, &mut eval);

            H[0] = add3_u32_2_2_4_unchecked(temp1.clone(), temp2, H[0].clone(), &mut eval);
            H[1] = add2_u32_2_4_unchecked(H[1].clone(), a_4, &mut eval);
            H[2] = add2_u32_2_4_unchecked(H[2].clone(), b, &mut eval);
            H[3] = add2_u32_2_4_unchecked(H[3].clone(), c, &mut eval);
            H[4] = add3_u32_2_4_4_unchecked(temp1, H[4].clone(), d.clone(), &mut eval);
            H[5] = add2_u32_2_4_unchecked(H[5].clone(), e_4, &mut eval);
            H[6] = add2_u32_2_4_unchecked(H[6].clone(), f, &mut eval);
            H[7] = add2_u32_2_4_unchecked(H[7].clone(), g, &mut eval);
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
    ) -> Fu32_2<E::F> {
        let one = E::EF::one();
        let [out0_lo, out0_hi, out1_lo, out1_hi, out2_0_lo, out2_1_lo, out2_0_hi, out2_1_hi] =
            std::array::from_fn(|_| eval.next_trace_mask());

        let xor_out2_lo = eval.next_trace_mask();
        let xor_out2_hi = eval.next_trace_mask();

        // Compute output of small sigma0 for first set of bits
        // BigSigma0 has special treatment because the set of bits that are affected by
        // both L0||H0||H1 and L1||L2||H2 is too large (we would need to XOR two words of 15 bits)
        match sigma {
            SigmaType::SmallSigma0 => {
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.small_sigma0_0,
                    -one.clone(),
                    &[
                        l1,
                        l2,
                        h2,
                        out0_lo.clone(),
                        out0_hi.clone(),
                        out2_0_lo.clone(),
                        out2_0_hi.clone(),
                    ],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.small_sigma0_1,
                    -one.clone(),
                    &[
                        l0,
                        h0,
                        h1,
                        out1_lo.clone(),
                        out1_hi.clone(),
                        out2_1_lo.clone(),
                        out2_1_hi.clone(),
                    ],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.xor_small_sigma0,
                    -one.clone(),
                    &[
                        out2_0_lo,
                        out2_0_hi,
                        out2_1_lo,
                        out2_1_hi,
                        xor_out2_lo.clone(),
                        xor_out2_hi.clone(),
                    ],
                ));
            }
            SigmaType::SmallSigma1 => {
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.small_sigma1_0,
                    -one.clone(),
                    &[
                        l0,
                        h0,
                        out0_lo.clone(),
                        out0_hi.clone(),
                        out2_0_lo.clone(),
                        out2_0_hi.clone(),
                    ],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.small_sigma1_1,
                    -one.clone(),
                    &[
                        l1,
                        l2,
                        h1,
                        h2,
                        out1_lo.clone(),
                        out1_hi.clone(),
                        out2_1_lo.clone(),
                        out2_1_hi.clone(),
                    ],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.xor_small_sigma1,
                    -one.clone(),
                    &[
                        out2_0_lo,
                        out2_0_hi,
                        out2_1_lo,
                        out2_1_hi,
                        xor_out2_lo.clone(),
                        xor_out2_hi.clone(),
                    ],
                ));
            }
            SigmaType::BigSigma0 => {
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.big_sigma0_0,
                    -one.clone(),
                    &[
                        l1,
                        l2,
                        h2,
                        out0_lo.clone(),
                        out0_hi.clone(),
                        out2_0_lo.clone(),
                        out2_0_hi.clone(),
                    ],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.big_sigma0_1,
                    -one.clone(),
                    &[
                        l0,
                        h0,
                        h1,
                        out1_lo.clone(),
                        out1_hi.clone(),
                        out2_1_lo.clone(),
                        out2_1_hi.clone(),
                    ],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.xor_big_sigma0_0,
                    -one.clone(),
                    &[out2_0_lo, out2_1_lo, xor_out2_lo.clone()],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.xor_big_sigma0_1,
                    -one.clone(),
                    &[out2_0_hi, out2_1_hi, xor_out2_hi.clone()],
                ));
            }
            SigmaType::BigSigma1 => {
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.big_sigma1_0,
                    -one.clone(),
                    &[
                        l0,
                        h0,
                        h1,
                        out0_lo.clone(),
                        out0_hi.clone(),
                        out2_0_lo.clone(),
                        out2_0_hi.clone(),
                    ],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.big_sigma1_1,
                    -one.clone(),
                    &[
                        l1,
                        l2,
                        h2,
                        out1_lo.clone(),
                        out1_hi.clone(),
                        out2_1_lo.clone(),
                        out2_1_hi.clone(),
                    ],
                ));
                eval.add_to_relation(RelationEntry::new(
                    &self.relations.xor_big_sigma1,
                    -one.clone(),
                    &[
                        out2_0_lo,
                        out2_0_hi,
                        out2_1_lo,
                        out2_1_hi,
                        xor_out2_lo.clone(),
                        xor_out2_hi.clone(),
                    ],
                ));
            }
        };

        // Add all limbs together to rebuild the 32-bit result
        let out0 = Fu32_2 {
            lo: out0_lo,
            hi: out0_hi,
        };
        let out1 = Fu32_2 {
            lo: out1_lo,
            hi: out1_hi,
        };
        let out2 = Fu32_2 {
            lo: xor_out2_lo,
            hi: xor_out2_hi,
        };
        let res = add3_u32_unchecked(out0, out1, out2, eval);

        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -one.clone(),
            &[res.lo.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -one,
            &[res.hi.clone()],
        ));

        res
    }

    fn ch<E: EvalAtRow>(
        &self,
        e: [E::F; 6],
        f: Fu32_4<E::F>,
        g: Fu32_4<E::F>,
        eval: &mut E,
    ) -> (Fu32_4<E::F>, Fu32_2<E::F>) {
        let one = E::EF::one();
        let two_pow_8 = E::F::from(M31::from(1 << 8));
        let e_4: Fu32_4<E::F> = Fu32_4::from_array(std::array::from_fn(|_| eval.next_trace_mask()));
        let ch: Fu32_4<E::F> = Fu32_4::from_array(std::array::from_fn(|_| eval.next_trace_mask()));

        eval.add_constraint(
            e_4.lo0.clone() + two_pow_8.clone() * e_4.lo1.clone()
                - e[0].clone()
                - e[1].clone()
                - e[2].clone(),
        );
        eval.add_constraint(
            e_4.hi0.clone() + two_pow_8 * e_4.hi1.clone()
                - e[3].clone()
                - e[4].clone()
                - e[5].clone(),
        );

        eval.add_to_relation(RelationEntry::new(
            &self.relations.ch,
            -one.clone(),
            &[
                e_4.lo0.clone(),
                f.lo0.clone(),
                g.lo0.clone(),
                ch.lo0.clone(),
            ],
        ));

        eval.add_to_relation(RelationEntry::new(
            &self.relations.ch,
            -one.clone(),
            &[
                e_4.lo1.clone(),
                f.lo1.clone(),
                g.lo1.clone(),
                ch.lo1.clone(),
            ],
        ));

        eval.add_to_relation(RelationEntry::new(
            &self.relations.ch,
            -one.clone(),
            &[
                e_4.hi0.clone(),
                f.hi0.clone(),
                g.hi0.clone(),
                ch.hi0.clone(),
            ],
        ));

        eval.add_to_relation(RelationEntry::new(
            &self.relations.ch,
            -one,
            &[e_4.hi1.clone(), f.hi1, g.hi1, ch.hi1.clone()],
        ));

        (e_4, ch.into())
    }

    fn maj<E: EvalAtRow>(
        &self,
        a: [E::F; 6],
        b: Fu32_4<E::F>,
        c: Fu32_4<E::F>,
        eval: &mut E,
    ) -> (Fu32_4<E::F>, Fu32_2<E::F>) {
        let one = E::EF::one();
        let two_pow_8 = E::F::from(M31::from(1 << 8));
        let a_4: Fu32_4<E::F> = Fu32_4::from_array(std::array::from_fn(|_| eval.next_trace_mask()));
        let maj: Fu32_4<E::F> = Fu32_4::from_array(std::array::from_fn(|_| eval.next_trace_mask()));

        eval.add_constraint(
            a_4.lo0.clone() + two_pow_8.clone() * a_4.lo1.clone()
                - a[0].clone()
                - a[1].clone()
                - a[2].clone(),
        );
        eval.add_constraint(
            a_4.hi0.clone() + two_pow_8 * a_4.hi1.clone()
                - a[3].clone()
                - a[4].clone()
                - a[5].clone(),
        );

        eval.add_to_relation(RelationEntry::new(
            &self.relations.maj,
            -one.clone(),
            &[
                a_4.lo0.clone(),
                b.lo0.clone(),
                c.lo0.clone(),
                maj.lo0.clone(),
            ],
        ));

        eval.add_to_relation(RelationEntry::new(
            &self.relations.maj,
            -one.clone(),
            &[
                a_4.lo1.clone(),
                b.lo1.clone(),
                c.lo1.clone(),
                maj.lo1.clone(),
            ],
        ));

        eval.add_to_relation(RelationEntry::new(
            &self.relations.maj,
            -one.clone(),
            &[
                a_4.hi0.clone(),
                b.hi0.clone(),
                c.hi0.clone(),
                maj.hi0.clone(),
            ],
        ));

        eval.add_to_relation(RelationEntry::new(
            &self.relations.maj,
            -one,
            &[a_4.hi1.clone(), b.hi1, c.hi1, maj.hi1.clone()],
        ));

        (a_4, maj.into())
    }
}

/// Adds two u32s, returning the sum.
/// Assumes a, b are properly range checked.
/// The caller is responsible for checking:
/// res.{l,h} not in [2^16, 2^17) or in [-2^16,0)
fn add2_u32_unchecked<E: EvalAtRow>(
    a: Fu32_2<E::F>,
    b: Fu32_2<E::F>,
    eval: &mut E,
) -> Fu32_2<E::F> {
    let sl = eval.next_trace_mask();
    let sh = eval.next_trace_mask();

    let carry_l = (a.lo + b.lo - sl.clone()) * E::F::from(INV16);
    eval.add_constraint(carry_l.clone() * carry_l.clone() - carry_l.clone());

    let carry_h = (a.hi + b.hi + carry_l - sh.clone()) * E::F::from(INV16);
    eval.add_constraint(carry_h.clone() * carry_h.clone() - carry_h);

    Fu32_2 { lo: sl, hi: sh }
}

/// Adds three u32s, returning the sum.
/// Assumes a, b, c are properly range checked.
fn add3_u32_unchecked<E: EvalAtRow>(
    a: Fu32_2<E::F>,
    b: Fu32_2<E::F>,
    c: Fu32_2<E::F>,
    eval: &mut E,
) -> Fu32_2<E::F> {
    let sl = eval.next_trace_mask();
    let sh = eval.next_trace_mask();

    let carry_l = (a.lo + b.lo + c.lo - sl.clone()) * E::F::from(INV16);
    eval.add_constraint(
        carry_l.clone() * (carry_l.clone() - E::F::one()) * (carry_l.clone() - E::F::from(TWO)),
    );

    let carry_h = (a.hi + b.hi + c.hi + carry_l - sh.clone()) * E::F::from(INV16);
    eval.add_constraint(
        carry_h.clone() * (carry_h.clone() - E::F::one()) * (carry_h - E::F::from(TWO)),
    );

    Fu32_2 { lo: sl, hi: sh }
}

/// Adds Fu32_2 and Fu32_4, returning the sum.
/// Assumes a, b are properly range checked.
fn add2_u32_2_4_unchecked<E: EvalAtRow>(
    a: Fu32_4<E::F>,
    b: Fu32_4<E::F>,
    eval: &mut E,
) -> Fu32_4<E::F> {
    let two_pow_8 = E::F::from(M31::from(1 << 8));
    let s = Fu32_4::from_array(std::array::from_fn(|_| eval.next_trace_mask()));

    let carry_l = (a.lo0 + two_pow_8.clone() * a.lo1 + b.lo0 + two_pow_8.clone() * b.lo1
        - (s.lo0.clone() + two_pow_8.clone() * s.lo1.clone()))
        * E::F::from(INV16);
    eval.add_constraint(carry_l.clone() * carry_l.clone() - carry_l.clone());

    let carry_h = (a.hi0 + two_pow_8.clone() * a.hi1 + b.hi0 + two_pow_8.clone() * b.hi1 + carry_l
        - (s.hi0.clone() + two_pow_8 * s.hi1.clone()))
        * E::F::from(INV16);
    eval.add_constraint(carry_h.clone() * carry_h.clone() - carry_h);

    s
}

/// Adds two Fu32_2s and one Fu32_4, returning the sum as Fu32_4.
/// Assumes a, b, c are properly range checked.
fn add3_u32_2_2_4_unchecked<E: EvalAtRow>(
    a: Fu32_2<E::F>,
    b: Fu32_2<E::F>,
    c: Fu32_4<E::F>,
    eval: &mut E,
) -> Fu32_4<E::F> {
    let two_pow_8 = E::F::from(M31::from(1 << 8));
    let s = Fu32_4::from_array(std::array::from_fn(|_| eval.next_trace_mask()));

    let carry_l = (a.lo + b.lo + c.lo0 + two_pow_8.clone() * c.lo1
        - (s.lo0.clone() + two_pow_8.clone() * s.lo1.clone()))
        * E::F::from(INV16);
    eval.add_constraint(
        carry_l.clone() * (carry_l.clone() - E::F::one()) * (carry_l.clone() - E::F::from(TWO)),
    );

    let carry_h = (a.hi + b.hi + c.hi0 + two_pow_8.clone() * c.hi1 + carry_l
        - (s.hi0.clone() + two_pow_8 * s.hi1.clone()))
        * E::F::from(INV16);
    eval.add_constraint(
        carry_h.clone() * (carry_h.clone() - E::F::one()) * (carry_h - E::F::from(TWO)),
    );

    s
}

/// Adds one Fu32_2 and two Fu32_4s, returning the sum as Fu32_4.
/// Assumes a, b, c are properly range checked.
fn add3_u32_2_4_4_unchecked<E: EvalAtRow>(
    a: Fu32_2<E::F>,
    b: Fu32_4<E::F>,
    c: Fu32_4<E::F>,
    eval: &mut E,
) -> Fu32_4<E::F> {
    let two_pow_8 = E::F::from(M31::from(1 << 8));
    let s = Fu32_4::from_array(std::array::from_fn(|_| eval.next_trace_mask()));

    let carry_l = (a.lo + b.lo0 + two_pow_8.clone() * b.lo1 + c.lo0 + two_pow_8.clone() * c.lo1
        - (s.lo0.clone() + two_pow_8.clone() * s.lo1.clone()))
        * E::F::from(INV16);
    eval.add_constraint(
        carry_l.clone() * (carry_l.clone() - E::F::one()) * (carry_l.clone() - E::F::from(TWO)),
    );

    let carry_h =
        (a.hi + b.hi0 + two_pow_8.clone() * b.hi1 + c.hi0 + two_pow_8.clone() * c.hi1 + carry_l
            - (s.hi0.clone() + two_pow_8 * s.hi1.clone()))
            * E::F::from(INV16);
    eval.add_constraint(
        carry_h.clone() * (carry_h.clone() - E::F::one()) * (carry_h - E::F::from(TWO)),
    );

    s
}
