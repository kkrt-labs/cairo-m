use zkhash::fields::m31::FpM31;
use zkhash::poseidon2::poseidon2::Poseidon2;
use zkhash::poseidon2::poseidon2_instance_m31::POSEIDON2_M31_16_PARAMS;

type Scalar = FpM31;

fn from_hex(hex_str: &str) -> Scalar {
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    let value = u64::from_str_radix(hex_str, 16).unwrap();
    Scalar::from(value)
}

/// Tests that the imported poseidon2 works fine (compared to reference test vector value)
#[test]
fn kats() {
    let poseidon2 = Poseidon2::new(&POSEIDON2_M31_16_PARAMS);
    let input: Vec<Scalar> = (0..16).map(|i| Scalar::from(i as u64)).collect();
    let perm = poseidon2.permutation(&input);
    assert_eq!(perm[0], from_hex("0x505d9689"));
    assert_eq!(perm[1], from_hex("0x3b64c904"));
    assert_eq!(perm[2], from_hex("0x79e2fd81"));
    assert_eq!(perm[3], from_hex("0x4ba8015f"));
    assert_eq!(perm[4], from_hex("0x24b6d2f5"));
    assert_eq!(perm[5], from_hex("0x23845add"));
    assert_eq!(perm[6], from_hex("0x521f4314"));
    assert_eq!(perm[7], from_hex("0x69dfb019"));
    assert_eq!(perm[8], from_hex("0x2aaae419"));
    assert_eq!(perm[9], from_hex("0x6cb4502c"));
    assert_eq!(perm[10], from_hex("0x6f7fa65a"));
    assert_eq!(perm[11], from_hex("0x75feff24"));
    assert_eq!(perm[12], from_hex("0x128d6587"));
    assert_eq!(perm[13], from_hex("0x515877e4"));
    assert_eq!(perm[14], from_hex("0x037f4dd7"));
    assert_eq!(perm[15], from_hex("0x134b427f"));
}
