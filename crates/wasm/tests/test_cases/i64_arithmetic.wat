(module
  (type (;0;) (func (param i64 i64) (result i64)))
  (func $i64_arithmetic_test (type 0) (param $a i64) (param $b i64) (result i64)
    local.get $a
    local.get $b
    i64.add
    local.get $a
    local.get $b
    i64.sub
    i64.add
    local.get $a
    local.get $b
    i64.sub
    i64.add
  )
  (memory (;0;) 1)
  (export "i64_arithmetic_test" (func $i64_arithmetic_test))
  (export "memory" (memory 0))
)
