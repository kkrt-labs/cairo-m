(module
  (memory $0 1)
  (export "memory" (memory $0))

  (global $a (mut i32) (i32.const 0))
  (global $b (mut i32) (i32.const 0))
  (global $c (mut i32) (i32.const 0))
  (global $d (mut i32) (i32.const 0))
  (global $e (mut i32) (i32.const 0))
  (global $f (mut i32) (i32.const 0))
  (global $g (mut i32) (i32.const 0))
  (global $h (mut i32) (i32.const 0))

  (func $i32.bswap
  (param $b i32)
  (result i32)
      
  ;; 2 get, 4 const, 5 bitwise

  (i32.or
    (get_local $b)
    (i32.const 0x00FF00FF)
    (i32.and)
    (i32.rotr (i32.const 8))

    (get_local $b)
    (i32.const 0xFF00FF00)
    (i32.and)
    (i32.rotl (i32.const 8))))

  (func $four_round 
    (param $w0 i32) (param $w1 i32) (param $w2 i32) (param $w3 i32)
    (param $k0 i32) (param $k1 i32) (param $k2 i32) (param $k3 i32)

    ;; precomputed values
    (local $T1 i32)
    (local $T2 i32)

    (local $ch_res i32)
    (local $maj_res i32)
    (local $big_sig0_res i32)
    (local $big_sig1_res i32)

    ;; precompute intermediate values
    ;; T1 = h + big_sig1(e) + ch(e, f, g) + K0 + W0
    ;; T2 = big_sig0(a) + Maj(a, b, c)

    ;; can compute 4 rounds independently

    (set_local $ch_res (i32.xor (i32.and (get_global $e) (get_global $f)) (i32.and (i32.xor (get_global $e) (i32.const -1)) (get_global $g))))
    (set_local $maj_res (i32.xor (i32.xor (i32.and (get_global $a) (get_global $b)) (i32.and (get_global $a) (get_global $c))) (i32.and (get_global $b) (get_global $c))))
    (set_local $big_sig0_res (i32.xor (i32.xor (i32.rotr (get_global $a) (i32.const 2)) (i32.rotr (get_global $a) (i32.const 13))) (i32.rotr (get_global $a) (i32.const 22))))
    (set_local $big_sig1_res (i32.xor (i32.xor (i32.rotr (get_global $e) (i32.const 6)) (i32.rotr (get_global $e) (i32.const 11))) (i32.rotr (get_global $e) (i32.const 25))))
    (set_local $T1 (i32.add (i32.add (i32.add (i32.add (get_global $h) (get_local $ch_res)) (get_local $big_sig1_res)) (get_local $w0)) (get_local $k0)))
    (set_local $T2 (i32.add (get_local $big_sig0_res) (get_local $maj_res)))

    (set_global $h (i32.add (get_global $d) (get_local $T1)))
    (set_global $d (i32.add (get_local $T1) (get_local $T2)))

    (set_local $ch_res (i32.xor (i32.and (get_global $h) (get_global $e)) (i32.and (i32.xor (get_global $h) (i32.const -1)) (get_global $f))))
    (set_local $maj_res (i32.xor (i32.xor (i32.and (get_global $d) (get_global $a)) (i32.and (get_global $d) (get_global $b))) (i32.and (get_global $a) (get_global $b))))
    (set_local $big_sig0_res (i32.xor (i32.xor (i32.rotr (get_global $d) (i32.const 2)) (i32.rotr (get_global $d) (i32.const 13))) (i32.rotr (get_global $d) (i32.const 22))))
    (set_local $big_sig1_res (i32.xor (i32.xor (i32.rotr (get_global $h) (i32.const 6)) (i32.rotr (get_global $h) (i32.const 11))) (i32.rotr (get_global $h) (i32.const 25))))
    (set_local $T1 (i32.add (i32.add (i32.add (i32.add (get_global $g) (get_local $ch_res)) (get_local $big_sig1_res)) (get_local $w1)) (get_local $k1)))
    (set_local $T2 (i32.add (get_local $big_sig0_res) (get_local $maj_res)))

    (set_global $g (i32.add (get_global $c) (get_local $T1)))
    (set_global $c (i32.add (get_local $T1) (get_local $T2)))

    (set_local $ch_res (i32.xor (i32.and (get_global $g) (get_global $h)) (i32.and (i32.xor (get_global $g) (i32.const -1)) (get_global $e))))
    (set_local $maj_res (i32.xor (i32.xor (i32.and (get_global $c) (get_global $d)) (i32.and (get_global $c) (get_global $a))) (i32.and (get_global $d) (get_global $a))))
    (set_local $big_sig0_res (i32.xor (i32.xor (i32.rotr (get_global $c) (i32.const 2)) (i32.rotr (get_global $c) (i32.const 13))) (i32.rotr (get_global $c) (i32.const 22))))
    (set_local $big_sig1_res (i32.xor (i32.xor (i32.rotr (get_global $g) (i32.const 6)) (i32.rotr (get_global $g) (i32.const 11))) (i32.rotr (get_global $g) (i32.const 25))))
    (set_local $T1 (i32.add (i32.add (i32.add (i32.add (get_global $f) (get_local $ch_res)) (get_local $big_sig1_res)) (get_local $w2)) (get_local $k2)))
    (set_local $T2 (i32.add (get_local $big_sig0_res) (get_local $maj_res)))

    (set_global $f (i32.add (get_global $b) (get_local $T1)))
    (set_global $b (i32.add (get_local $T1) (get_local $T2)))

    (set_local $ch_res (i32.xor (i32.and (get_global $f) (get_global $g)) (i32.and (i32.xor (get_global $f) (i32.const -1)) (get_global $h))))
    (set_local $maj_res (i32.xor (i32.xor (i32.and (get_global $b) (get_global $c)) (i32.and (get_global $b) (get_global $d))) (i32.and (get_global $c) (get_global $d))))
    (set_local $big_sig0_res (i32.xor (i32.xor (i32.rotr (get_global $b) (i32.const 2)) (i32.rotr (get_global $b) (i32.const 13))) (i32.rotr (get_global $b) (i32.const 22))))
    (set_local $big_sig1_res (i32.xor (i32.xor (i32.rotr (get_global $f) (i32.const 6)) (i32.rotr (get_global $f) (i32.const 11))) (i32.rotr (get_global $f) (i32.const 25))))
    (set_local $T1 (i32.add (i32.add (i32.add (i32.add (get_global $e) (get_local $ch_res)) (get_local $big_sig1_res)) (get_local $w3)) (get_local $k3)))
    (set_local $T2 (i32.add (get_local $big_sig0_res) (get_local $maj_res)))

    (set_global $e (i32.add (get_global $a) (get_local $T1)))
    (set_global $a (i32.add (get_local $T1) (get_local $T2))))

  (func $expand
    (param $a i32) (param $b i32) (param $c i32) (param $d i32)
    (result i32)

    (i32.add
      (i32.add
        (i32.add
          (i32.xor
            (i32.xor
              (i32.rotr (get_local $a) (i32.const 17))
              (i32.rotr (get_local $a) (i32.const 19)))
            (i32.shr_u (get_local $a) (i32.const 10)))
          (get_local $b))
        (i32.xor
          (i32.xor
            (i32.rotr (get_local $c) (i32.const 7))
            (i32.rotr (get_local $c) (i32.const 18)))
          (i32.shr_u (get_local $c) (i32.const 3)))
        (get_local $d))))

  (func $compress
    (param $ctx i32)

    (param $w0 i32) (param $w1 i32) (param $w2 i32) (param $w3 i32)
    (param $w4 i32) (param $w5 i32) (param $w6 i32) (param $w7 i32)
    (param $w8 i32) (param $w9 i32) (param $w10 i32) (param $w11 i32)
    (param $w12 i32) (param $w13 i32) (param $w14 i32) (param $w15 i32)

    ;; expanded message schedule
    (local $w16 i32) (local $w17 i32) (local $w18 i32) (local $w19 i32) 
    (local $w20 i32) (local $w21 i32) (local $w22 i32) (local $w23 i32)
    (local $w24 i32) (local $w25 i32) (local $w26 i32) (local $w27 i32) 
    (local $w28 i32) (local $w29 i32) (local $w30 i32) (local $w31 i32)
    (local $w32 i32) (local $w33 i32) (local $w34 i32) (local $w35 i32) 
    (local $w36 i32) (local $w37 i32) (local $w38 i32) (local $w39 i32)
    (local $w40 i32) (local $w41 i32) (local $w42 i32) (local $w43 i32) 
    (local $w44 i32) (local $w45 i32) (local $w46 i32) (local $w47 i32)
    (local $w48 i32) (local $w49 i32) (local $w50 i32) (local $w51 i32) 
    (local $w52 i32) (local $w53 i32) (local $w54 i32) (local $w55 i32)
    (local $w56 i32) (local $w57 i32) (local $w58 i32) (local $w59 i32) 
    (local $w60 i32) (local $w61 i32) (local $w62 i32) (local $w63 i32)

    ;;  store inital state
    (if (i32.eqz (i32.load offset=104 (get_local $ctx)))
        (then
            (i32.store offset=0  (get_local $ctx) (i32.const 0x6a09e667))
            (i32.store offset=4  (get_local $ctx) (i32.const 0xbb67ae85))
            (i32.store offset=8  (get_local $ctx) (i32.const 0x3c6ef372))
            (i32.store offset=12 (get_local $ctx) (i32.const 0xa54ff53a))
            (i32.store offset=16 (get_local $ctx) (i32.const 0x510e527f))
            (i32.store offset=20 (get_local $ctx) (i32.const 0x9b05688c))
            (i32.store offset=24 (get_local $ctx) (i32.const 0x1f83d9ab))
            (i32.store offset=28 (get_local $ctx) (i32.const 0x5be0cd19))
            (i32.store offset=104 (get_local $ctx) (i32.const 1))))

    ;; load previous hash state into registers
    (set_global $a (i32.load offset=0  (get_local $ctx)))
    (set_global $b (i32.load offset=4  (get_local $ctx)))
    (set_global $c (i32.load offset=8  (get_local $ctx)))
    (set_global $d (i32.load offset=12 (get_local $ctx)))
    (set_global $e (i32.load offset=16 (get_local $ctx)))
    (set_global $f (i32.load offset=20 (get_local $ctx)))
    (set_global $g (i32.load offset=24 (get_local $ctx)))
    (set_global $h (i32.load offset=28 (get_local $ctx)))

    (set_local $w0  (call $i32.bswap (get_local $w0 )))
    (set_local $w1  (call $i32.bswap (get_local $w1 )))
    (set_local $w2  (call $i32.bswap (get_local $w2 )))
    (set_local $w3  (call $i32.bswap (get_local $w3 )))
    (set_local $w4  (call $i32.bswap (get_local $w4 )))
    (set_local $w5  (call $i32.bswap (get_local $w5 )))
    (set_local $w6  (call $i32.bswap (get_local $w6 )))
    (set_local $w7  (call $i32.bswap (get_local $w7 )))
    (set_local $w8  (call $i32.bswap (get_local $w8 )))
    (set_local $w9  (call $i32.bswap (get_local $w9 )))
    (set_local $w10 (call $i32.bswap (get_local $w10)))
    (set_local $w11 (call $i32.bswap (get_local $w11)))
    (set_local $w12 (call $i32.bswap (get_local $w12)))
    (set_local $w13 (call $i32.bswap (get_local $w13)))
    (set_local $w14 (call $i32.bswap (get_local $w14)))
    (set_local $w15 (call $i32.bswap (get_local $w15)))

    (call $four_round (get_local $w0)  (get_local $w1)  (get_local $w2)  (get_local $w3)  (i32.const 0x428a2f98) (i32.const 0x71374491) (i32.const 0xb5c0fbcf) (i32.const 0xe9b5dba5))
    (call $four_round (get_local $w4)  (get_local $w5)  (get_local $w6)  (get_local $w7)  (i32.const 0x3956c25b) (i32.const 0x59f111f1) (i32.const 0x923f82a4) (i32.const 0xab1c5ed5))
    (call $four_round (get_local $w8)  (get_local $w9)  (get_local $w10) (get_local $w11) (i32.const 0xd807aa98) (i32.const 0x12835b01) (i32.const 0x243185be) (i32.const 0x550c7dc3))
    (call $four_round (get_local $w12) (get_local $w13) (get_local $w14) (get_local $w15) (i32.const 0x72be5d74) (i32.const 0x80deb1fe) (i32.const 0x9bdc06a7) (i32.const 0xc19bf174))

    (set_local $w0  (call $expand (get_local $w14) (get_local $w9)  (get_local $w1)  (get_local $w0)))
    (set_local $w1  (call $expand (get_local $w15) (get_local $w10) (get_local $w2)  (get_local $w1)))
    (set_local $w2  (call $expand (get_local $w0 ) (get_local $w11) (get_local $w3)  (get_local $w2)))
    (set_local $w3  (call $expand (get_local $w1 ) (get_local $w12) (get_local $w4)  (get_local $w3)))
    (set_local $w4  (call $expand (get_local $w2 ) (get_local $w13) (get_local $w5)  (get_local $w4)))
    (set_local $w5  (call $expand (get_local $w3 ) (get_local $w14) (get_local $w6)  (get_local $w5)))
    (set_local $w6  (call $expand (get_local $w4 ) (get_local $w15) (get_local $w7)  (get_local $w6)))
    (set_local $w7  (call $expand (get_local $w5 ) (get_local $w0)  (get_local $w8)  (get_local $w7)))
    (set_local $w8  (call $expand (get_local $w6 ) (get_local $w1)  (get_local $w9)  (get_local $w8)))
    (set_local $w9  (call $expand (get_local $w7 ) (get_local $w2)  (get_local $w10) (get_local $w9)))
    (set_local $w10 (call $expand (get_local $w8 ) (get_local $w3)  (get_local $w11) (get_local $w10)))
    (set_local $w11 (call $expand (get_local $w9 ) (get_local $w4)  (get_local $w12) (get_local $w11)))
    (set_local $w12 (call $expand (get_local $w10) (get_local $w5)  (get_local $w13) (get_local $w12)))
    (set_local $w13 (call $expand (get_local $w11) (get_local $w6)  (get_local $w14) (get_local $w13)))
    (set_local $w14 (call $expand (get_local $w12) (get_local $w7)  (get_local $w15) (get_local $w14)))
    (set_local $w15 (call $expand (get_local $w13) (get_local $w8)  (get_local $w0)  (get_local $w15)))

    (call $four_round (get_local $w0)  (get_local $w1)  (get_local $w2)  (get_local $w3)  (i32.const 0xe49b69c1) (i32.const 0xefbe4786) (i32.const 0x0fc19dc6) (i32.const 0x240ca1cc))
    (call $four_round (get_local $w4)  (get_local $w5)  (get_local $w6)  (get_local $w7)  (i32.const 0x2de92c6f) (i32.const 0x4a7484aa) (i32.const 0x5cb0a9dc) (i32.const 0x76f988da))
    (call $four_round (get_local $w8)  (get_local $w9)  (get_local $w10) (get_local $w11) (i32.const 0x983e5152) (i32.const 0xa831c66d) (i32.const 0xb00327c8) (i32.const 0xbf597fc7))
    (call $four_round (get_local $w12) (get_local $w13) (get_local $w14) (get_local $w15) (i32.const 0xc6e00bf3) (i32.const 0xd5a79147) (i32.const 0x06ca6351) (i32.const 0x14292967))

    (set_local $w0  (call $expand (get_local $w14) (get_local $w9)  (get_local $w1)  (get_local $w0)))
    (set_local $w1  (call $expand (get_local $w15) (get_local $w10) (get_local $w2)  (get_local $w1)))
    (set_local $w2  (call $expand (get_local $w0 ) (get_local $w11) (get_local $w3)  (get_local $w2)))
    (set_local $w3  (call $expand (get_local $w1 ) (get_local $w12) (get_local $w4)  (get_local $w3)))
    (set_local $w4  (call $expand (get_local $w2 ) (get_local $w13) (get_local $w5)  (get_local $w4)))
    (set_local $w5  (call $expand (get_local $w3 ) (get_local $w14) (get_local $w6)  (get_local $w5)))
    (set_local $w6  (call $expand (get_local $w4 ) (get_local $w15) (get_local $w7)  (get_local $w6)))
    (set_local $w7  (call $expand (get_local $w5 ) (get_local $w0)  (get_local $w8)  (get_local $w7)))
    (set_local $w8  (call $expand (get_local $w6 ) (get_local $w1)  (get_local $w9)  (get_local $w8)))
    (set_local $w9  (call $expand (get_local $w7 ) (get_local $w2)  (get_local $w10) (get_local $w9)))
    (set_local $w10 (call $expand (get_local $w8 ) (get_local $w3)  (get_local $w11) (get_local $w10)))
    (set_local $w11 (call $expand (get_local $w9 ) (get_local $w4)  (get_local $w12) (get_local $w11)))
    (set_local $w12 (call $expand (get_local $w10) (get_local $w5)  (get_local $w13) (get_local $w12)))
    (set_local $w13 (call $expand (get_local $w11) (get_local $w6)  (get_local $w14) (get_local $w13)))
    (set_local $w14 (call $expand (get_local $w12) (get_local $w7)  (get_local $w15) (get_local $w14)))
    (set_local $w15 (call $expand (get_local $w13) (get_local $w8)  (get_local $w0)  (get_local $w15)))

    (call $four_round (get_local $w0)  (get_local $w1)  (get_local $w2)  (get_local $w3)  (i32.const 0x27b70a85) (i32.const 0x2e1b2138) (i32.const 0x4d2c6dfc) (i32.const 0x53380d13))
    (call $four_round (get_local $w4)  (get_local $w5)  (get_local $w6)  (get_local $w7)  (i32.const 0x650a7354) (i32.const 0x766a0abb) (i32.const 0x81c2c92e) (i32.const 0x92722c85))
    (call $four_round (get_local $w8)  (get_local $w9)  (get_local $w10) (get_local $w11) (i32.const 0xa2bfe8a1) (i32.const 0xa81a664b) (i32.const 0xc24b8b70) (i32.const 0xc76c51a3))
    (call $four_round (get_local $w12) (get_local $w13) (get_local $w14) (get_local $w15) (i32.const 0xd192e819) (i32.const 0xd6990624) (i32.const 0xf40e3585) (i32.const 0x106aa070))

    (set_local $w0  (call $expand (get_local $w14) (get_local $w9)  (get_local $w1)  (get_local $w0)))
    (set_local $w1  (call $expand (get_local $w15) (get_local $w10) (get_local $w2)  (get_local $w1)))
    (set_local $w2  (call $expand (get_local $w0 ) (get_local $w11) (get_local $w3)  (get_local $w2)))
    (set_local $w3  (call $expand (get_local $w1 ) (get_local $w12) (get_local $w4)  (get_local $w3)))
    (set_local $w4  (call $expand (get_local $w2 ) (get_local $w13) (get_local $w5)  (get_local $w4)))
    (set_local $w5  (call $expand (get_local $w3 ) (get_local $w14) (get_local $w6)  (get_local $w5)))
    (set_local $w6  (call $expand (get_local $w4 ) (get_local $w15) (get_local $w7)  (get_local $w6)))
    (set_local $w7  (call $expand (get_local $w5 ) (get_local $w0)  (get_local $w8)  (get_local $w7)))
    (set_local $w8  (call $expand (get_local $w6 ) (get_local $w1)  (get_local $w9)  (get_local $w8)))
    (set_local $w9  (call $expand (get_local $w7 ) (get_local $w2)  (get_local $w10) (get_local $w9)))
    (set_local $w10 (call $expand (get_local $w8 ) (get_local $w3)  (get_local $w11) (get_local $w10)))
    (set_local $w11 (call $expand (get_local $w9 ) (get_local $w4)  (get_local $w12) (get_local $w11)))
    (set_local $w12 (call $expand (get_local $w10) (get_local $w5)  (get_local $w13) (get_local $w12)))
    (set_local $w13 (call $expand (get_local $w11) (get_local $w6)  (get_local $w14) (get_local $w13)))
    (set_local $w14 (call $expand (get_local $w12) (get_local $w7)  (get_local $w15) (get_local $w14)))
    (set_local $w15 (call $expand (get_local $w13) (get_local $w8)  (get_local $w0)  (get_local $w15)))

    (call $four_round (get_local $w0)  (get_local $w1)  (get_local $w2)  (get_local $w3)  (i32.const 0x19a4c116) (i32.const 0x1e376c08) (i32.const 0x2748774c) (i32.const 0x34b0bcb5))
    (call $four_round (get_local $w4)  (get_local $w5)  (get_local $w6)  (get_local $w7)  (i32.const 0x391c0cb3) (i32.const 0x4ed8aa4a) (i32.const 0x5b9cca4f) (i32.const 0x682e6ff3))
    (call $four_round (get_local $w8)  (get_local $w9)  (get_local $w10) (get_local $w11) (i32.const 0x748f82ee) (i32.const 0x78a5636f) (i32.const 0x84c87814) (i32.const 0x8cc70208))
    (call $four_round (get_local $w12) (get_local $w13) (get_local $w14) (get_local $w15) (i32.const 0x90befffa) (i32.const 0xa4506ceb) (i32.const 0xbef9a3f7) (i32.const 0xc67178f2))

    ;; store hash values
    (i32.store offset=0  (get_local $ctx) (i32.add (i32.load offset=0  (get_local $ctx)) (get_global $a)))
    (i32.store offset=4  (get_local $ctx) (i32.add (i32.load offset=4  (get_local $ctx)) (get_global $b)))
    (i32.store offset=8  (get_local $ctx) (i32.add (i32.load offset=8  (get_local $ctx)) (get_global $c)))
    (i32.store offset=12 (get_local $ctx) (i32.add (i32.load offset=12 (get_local $ctx)) (get_global $d)))
    (i32.store offset=16 (get_local $ctx) (i32.add (i32.load offset=16 (get_local $ctx)) (get_global $e)))
    (i32.store offset=20 (get_local $ctx) (i32.add (i32.load offset=20 (get_local $ctx)) (get_global $f)))
    (i32.store offset=24 (get_local $ctx) (i32.add (i32.load offset=24 (get_local $ctx)) (get_global $g)))
    (i32.store offset=28 (get_local $ctx) (i32.add (i32.load offset=28 (get_local $ctx)) (get_global $h))))
        
  (func $sha256 (export "sha256") (param $ctx i32) (param $roi i32) (param $length i32) (param $final i32)
    ;;    schema  208 bytes
    ;;     0..32  hash state
    ;;    32..40  number of bytes read across all updates (128bit)
    ;;   40..104  store words between updates
    ;;  104..108  init flag

    (local $bytes_read i64)
    (local $last_word i32)
    (local $tail i32)

    ;; expanded message schedule
    (local $w0 i32)  (local $w1 i32)  (local $w2 i32)  (local $w3 i32)  
    (local $w4 i32)  (local $w5 i32)  (local $w6 i32)  (local $w7 i32) 
    (local $w8 i32)  (local $w9 i32)  (local $w10 i32) (local $w11 i32)
    (local $w12 i32) (local $w13 i32) (local $w14 i32) (local $w15 i32)

    (set_local $bytes_read (i64.load offset=32 (get_local $ctx)))
    (set_local $tail (i32.add (i32.and (i32.wrap/i64 (get_local $bytes_read)) (i32.const 0x3f)) (get_local $length)))

    ;; load current block position
    (set_local $bytes_read (i64.add (get_local $bytes_read) (i64.extend_u/i32 (get_local $length))))
    (i64.store offset=32 (get_local $ctx) (get_local $bytes_read))

    (block $finish
      (set_local $w0  (i32.load offset=40 (get_local $ctx)))
      (set_local $w1  (i32.load offset=44 (get_local $ctx)))
      (set_local $w2  (i32.load offset=48 (get_local $ctx)))
      (set_local $w3  (i32.load offset=52 (get_local $ctx)))
      (set_local $w4  (i32.load offset=56 (get_local $ctx)))
      (set_local $w5  (i32.load offset=60 (get_local $ctx)))
      (set_local $w6  (i32.load offset=64 (get_local $ctx)))
      (set_local $w7  (i32.load offset=68 (get_local $ctx)))
      (set_local $w8  (i32.load offset=72 (get_local $ctx)))
      (set_local $w9  (i32.load offset=76 (get_local $ctx)))
      (set_local $w10 (i32.load offset=80 (get_local $ctx)))
      (set_local $w11 (i32.load offset=84 (get_local $ctx)))
      (set_local $w12 (i32.load offset=88 (get_local $ctx)))
      (set_local $w13 (i32.load offset=92 (get_local $ctx)))
      (set_local $w14 (i32.load offset=96 (get_local $ctx)))
      (set_local $w15 (i32.load offset=100 (get_local $ctx)))

      (tee_local $tail (i32.sub (get_local $tail) (i32.const 64)))
      (i32.const 0)
      (i32.lt_s)
      (br_if $finish)

      (get_local $ctx)
      (get_local $w0 )
      (get_local $w1 )
      (get_local $w2 )
      (get_local $w3 )
      (get_local $w4 )
      (get_local $w5 )
      (get_local $w6 )
      (get_local $w7 )
      (get_local $w8 )
      (get_local $w9 )
      (get_local $w10)
      (get_local $w11)
      (get_local $w12)
      (get_local $w13)
      (get_local $w14)
      (get_local $w15)
      (call $compress)

      (loop $rest_of_input
        (set_local $w0  (i32.load offset=0  (get_local $roi)))
        (set_local $w1  (i32.load offset=4  (get_local $roi)))
        (set_local $w2  (i32.load offset=8  (get_local $roi)))
        (set_local $w3  (i32.load offset=12 (get_local $roi)))
        (set_local $w4  (i32.load offset=16 (get_local $roi)))
        (set_local $w5  (i32.load offset=20 (get_local $roi)))
        (set_local $w6  (i32.load offset=24 (get_local $roi)))
        (set_local $w7  (i32.load offset=28 (get_local $roi)))
        (set_local $w8  (i32.load offset=32 (get_local $roi)))
        (set_local $w9  (i32.load offset=36 (get_local $roi)))
        (set_local $w10 (i32.load offset=40 (get_local $roi)))
        (set_local $w11 (i32.load offset=44 (get_local $roi)))
        (set_local $w12 (i32.load offset=48 (get_local $roi)))
        (set_local $w13 (i32.load offset=52 (get_local $roi)))
        (set_local $w14 (i32.load offset=56 (get_local $roi)))
        (set_local $w15 (i32.load offset=60 (get_local $roi)))

        (set_local $roi (i32.add (get_local $roi) (i32.const 64)))

        (tee_local $tail (i32.sub (get_local $tail) (i32.const 64)))
        (i32.const 0)
        (i32.lt_s)
        (if
          (then
            (i32.store offset=40 (get_local $ctx) (get_local $w0))
            (i32.store offset=44 (get_local $ctx) (get_local $w1))
            (i32.store offset=48 (get_local $ctx) (get_local $w2))
            (i32.store offset=52 (get_local $ctx) (get_local $w3))
            (i32.store offset=56 (get_local $ctx) (get_local $w4))
            (i32.store offset=60 (get_local $ctx) (get_local $w5))
            (i32.store offset=64 (get_local $ctx) (get_local $w6))
            (i32.store offset=68 (get_local $ctx) (get_local $w7))
            (i32.store offset=72 (get_local $ctx) (get_local $w8))
            (i32.store offset=76 (get_local $ctx) (get_local $w9))
            (i32.store offset=80 (get_local $ctx) (get_local $w10))
            (i32.store offset=84 (get_local $ctx) (get_local $w11))
            (i32.store offset=88 (get_local $ctx) (get_local $w12))
            (i32.store offset=92 (get_local $ctx) (get_local $w13))
            (i32.store offset=96 (get_local $ctx) (get_local $w14))
            (i32.store offset=100 (get_local $ctx) (get_local $w15))
            (br $finish)))

        (get_local $ctx)
        (get_local $w0 )
        (get_local $w1 )
        (get_local $w2 )
        (get_local $w3 )
        (get_local $w4 )
        (get_local $w5 )
        (get_local $w6 )
        (get_local $w7 )
        (get_local $w8 )
        (get_local $w9 )
        (get_local $w10)
        (get_local $w11)
        (get_local $w12)
        (get_local $w13)
        (get_local $w14)
        (get_local $w15)
        (call $compress)

        (br $rest_of_input)))

    (if (i32.eq (get_local $final) (i32.const 1))
      (then
        (set_local $tail (i32.and (i32.wrap/i64 (get_local $bytes_read)) (i32.const 0x3f)))
        (set_local $last_word (i32.shl (i32.const 0x80) (i32.shl (i32.and (get_local $tail) (i32.const 0x3)) (i32.const 3))))

        (block $pad_end
                (block $13
                    (block $12
                        (block $11
                            (block $10
                                (block $9
                                    (block $8
                                        (block $7
                                            (block $6
                                                (block $5
                                                    (block $4
                                                        (block $3
                                                            (block $2
                                                                (block $1
                                                                    (block $0
                                                                        (block $15
                                                                            (block $14
                                                                                (block $switch
                                                                                    (get_local $tail)
                                                                                    (i32.const 2)
                                                                                    (i32.shr_u)
                                                                                    (br_table $0 $1 $2 $3 $4 $5 $6 $7 $8 $9 $10 $11 $12 $13 $14 $15)))
                                                                                
                                                                                (get_local $last_word)
                                                                                (get_local $w14)
                                                                                (i32.or)
                                                                                (set_local $w14)
                                                                                (set_local $last_word (i32.const 0)))
                                                                            
                                                                            (get_local $last_word)
                                                                            (get_local $w15)
                                                                            (i32.or)
                                                                            (set_local $w15)
                                                                            (set_local $last_word (i32.const 0))

                                                                            ;; compress
                                                                            (get_local $ctx)
                                                                            (get_local $w0)
                                                                            (get_local $w1)
                                                                            (get_local $w2)
                                                                            (get_local $w3)
                                                                            (get_local $w4)
                                                                            (get_local $w5)
                                                                            (get_local $w6)
                                                                            (get_local $w7)
                                                                            (get_local $w8)
                                                                            (get_local $w9)
                                                                            (get_local $w10)
                                                                            (get_local $w11)
                                                                            (get_local $w12)
                                                                            (get_local $w13)
                                                                            (get_local $w14)
                                                                            (get_local $w15)
                                                                            (call $compress)

                                                                            (i64.store offset=32 (get_local $ctx) (get_local $bytes_read))

                                                                            ;; zero out words
                                                                            (set_local $w0  (i32.const 0))
                                                                            (set_local $w1  (i32.const 0))
                                                                            (set_local $w2  (i32.const 0))
                                                                            (set_local $w3  (i32.const 0))
                                                                            (set_local $w4  (i32.const 0))
                                                                            (set_local $w5  (i32.const 0))
                                                                            (set_local $w6  (i32.const 0))
                                                                            (set_local $w7  (i32.const 0))
                                                                            (set_local $w8  (i32.const 0))
                                                                            (set_local $w9  (i32.const 0))
                                                                            (set_local $w10 (i32.const 0))
                                                                            (set_local $w11 (i32.const 0))
                                                                            (set_local $w12 (i32.const 0))
                                                                            (set_local $w13 (i32.const 0))
                                                                            (set_local $w14 (i32.const 0))
                                                                            (set_local $w15 (i32.const 0)))
                                                                        
                                                                        (get_local $last_word)
                                                                        (get_local $w0)
                                                                        (i32.or)
                                                                        (set_local $w0)
                                                                        (set_local $last_word (i32.const 0)))
                                                                    
                                                                    (get_local $last_word)
                                                                    (get_local $w1)
                                                                    (i32.or)
                                                                    (set_local $w1)
                                                                    (set_local $last_word (i32.const 0)))
                                                                
                                                                (get_local $last_word)
                                                                (get_local $w2)
                                                                (i32.or)
                                                                (set_local $w2)
                                                                (set_local $last_word (i32.const 0)))
                                                            
                                                            (get_local $last_word)
                                                            (get_local $w3)
                                                            (i32.or)
                                                            (set_local $w3)
                                                            (set_local $last_word (i32.const 0)))
                                                        
                                                        (get_local $last_word)
                                                        (get_local $w4)
                                                        (i32.or)
                                                        (set_local $w4)
                                                        (set_local $last_word (i32.const 0)))
                                                    
                                                    (get_local $last_word)
                                                    (get_local $w5)
                                                    (i32.or)
                                                    (set_local $w5)
                                                    (set_local $last_word (i32.const 0)))
                                                
                                                (get_local $last_word)
                                                (get_local $w6)
                                                (i32.or)
                                                (set_local $w6)
                                                (set_local $last_word (i32.const 0)))
                                            
                                            (get_local $last_word)
                                            (get_local $w7)
                                            (i32.or)
                                            (set_local $w7)
                                            (set_local $last_word (i32.const 0)))
                                        
                                        (get_local $last_word)
                                        (get_local $w8)
                                        (i32.or)
                                        (set_local $w8)
                                        (set_local $last_word (i32.const 0)))
                                    
                                    (get_local $last_word)
                                    (get_local $w9)
                                    (i32.or)
                                    (set_local $w9)
                                    (set_local $last_word (i32.const 0)))
                                
                                (get_local $last_word)
                                (get_local $w10)
                                (i32.or)
                                (set_local $w10)
                                (set_local $last_word (i32.const 0)))
                            
                            (get_local $last_word)
                            (get_local $w11)
                            (i32.or)
                            (set_local $w11)
                            (set_local $last_word (i32.const 0)))
                        
                        (get_local $last_word)
                        (get_local $w12)
                        (i32.or)
                        (set_local $w12)
                        (set_local $last_word (i32.const 0)))
                    
                    (get_local $last_word)
                    (get_local $w13)
                    (i32.or)
                    (set_local $w13)
                    (set_local $last_word (i32.const 0)))
            
            (set_local $w14 (call $i32.bswap (i32.wrap/i64 (i64.shr_u (get_local $bytes_read) (i64.const 29)))))
            (set_local $w15 (call $i32.bswap (i32.wrap/i64 (i64.shl (get_local $bytes_read) (i64.const 3)))))

            (get_local $ctx)
            (get_local $w0)
            (get_local $w1)
            (get_local $w2)
            (get_local $w3)
            (get_local $w4)
            (get_local $w5)
            (get_local $w6)
            (get_local $w7)
            (get_local $w8)
            (get_local $w9)
            (get_local $w10)
            (get_local $w11)
            (get_local $w12)
            (get_local $w13)
            (get_local $w14)
            (get_local $w15)
            (call $compress)

            (get_local $ctx)
            (i32.load offset=0 (get_local $ctx))
            (call $i32.bswap)
            (i32.store offset=0)

            (get_local $ctx)
            (i32.load offset=4 (get_local $ctx))
            (call $i32.bswap)
            (i32.store offset=4)

            (get_local $ctx)
            (i32.load offset=8 (get_local $ctx))
            (call $i32.bswap)
            (i32.store offset=8)

            (get_local $ctx)
            (i32.load offset=12 (get_local $ctx))
            (call $i32.bswap)
            (i32.store offset=12)

            (get_local $ctx)
            (i32.load offset=16 (get_local $ctx))
            (call $i32.bswap)
            (i32.store offset=16)

            (get_local $ctx)
            (i32.load offset=20 (get_local $ctx))
            (call $i32.bswap)
            (i32.store offset=20)

            (get_local $ctx)
            (i32.load offset=24 (get_local $ctx))
            (call $i32.bswap)
            (i32.store offset=24)

            (get_local $ctx)
            (i32.load offset=28 (get_local $ctx))
            (call $i32.bswap)
            (i32.store offset=28)))))
