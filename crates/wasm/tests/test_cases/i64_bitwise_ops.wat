(module
  (type (;0;) (func (param i64 i64) (result i64)))
  (func $i64_bitwise_test (type 0) (param $a i64) (param $b i64) (result i64)
    (local $temp1 i64)
    (local $temp2 i64)
    (local $temp3 i64)
    (local $result i64)

    ;; Test i64.const operations
    i64.const 0x123456789ABCDEF0  ;; First constant
    local.set $temp1

    i64.const 0xFEDCBA0987654321  ;; Second constant
    local.set $temp2

    ;; Test i64.and: temp1 & temp2
    local.get $temp1
    local.get $temp2
    i64.and
    local.set $temp3

    ;; Test i64.or: (temp1 & temp2) | param_a
    local.get $temp3
    local.get $a
    i64.or
    local.set $result

    ;; Test i64.xor: result ^ param_b
    local.get $result
    local.get $b
    i64.xor
    local.set $result

    ;; Final combination: result & 0xFFFFFFFFFFFFFFFF (identity)
    local.get $result
    i64.const 0xFFFFFFFFFFFFFFFF
    i64.and

    ;; Return the final result
  )
  (export "i64_bitwise_test" (func $i64_bitwise_test))
)
