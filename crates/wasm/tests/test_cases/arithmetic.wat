(module
  ;; Test i32 arithmetic operations
  (func $test_add (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.add
  )

  (func $test_sub (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.sub
  )

  (func $test_mul (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.mul
  )

  (func $test_div_u (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.div_u
  )

  ;; Combined test function that tests all operations
  (func $arithmetic_test (param $a i32) (param $b i32) (result i32)
    ;; Test: (a + b) * (a - b) / (a + 1) using signed division
    local.get $a
    local.get $b
    i32.add
    local.get $a
    local.get $b
    i32.sub
    i32.mul
    local.get $a
    i32.const 1
    i32.add
    i32.div_u
  )

  (export "test_add" (func $test_add))
  (export "test_sub" (func $test_sub))
  (export "test_mul" (func $test_mul))
  (export "test_div_u" (func $test_div_u))
  (export "arithmetic_test" (func $arithmetic_test))
)
