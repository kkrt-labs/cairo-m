(module
    (func $i32_add (param $a i32) (param $b i32) (result i32)
        local.get $a
        local.get $b
        i32.add
    )

    (func $i32_sub (param $a i32) (param $b i32) (result i32)
        local.get $a
        local.get $b
        i32.sub
    )

    (func $i32_mul (param $a i32) (param $b i32) (result i32)
        local.get $a
        local.get $b
        i32.mul
    )

    (func $i32_div_u (param $a i32) (param $b i32) (result i32)
        local.get $a
        local.get $b
        i32.div_u
    )

    (export "i32_add" (func $i32_add))
    (export "i32_sub" (func $i32_sub))
    (export "i32_mul" (func $i32_mul))
    (export "i32_div_u" (func $i32_div_u))
)
