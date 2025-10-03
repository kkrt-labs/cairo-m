(module
  (func $i32_and (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.and
  )

  (func $i32_or (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.or
  )

  (func $i32_xor (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.xor
  )

  (export "i32_and" (func $i32_and))
  (export "i32_or" (func $i32_or))
  (export "i32_xor" (func $i32_xor))
)
