(module
 (func $nested_loop (param $0 i32) (result i32)
  (local $1 i32)
  (local $2 i32)
  (local $3 i32)
  (local.set $1
   (i32.const 0)
  )
  (local.set $3
   (i32.const 0)
  )
  (loop $label$1
   (i32.lt_u
    (local.get $1)
    (local.get $0)
   )
   (if
    (then
     (local.set $2
      (i32.const 0)
     )
     (loop $label$3
      (i32.lt_u
       (local.get $2)
       (local.get $1)
      )
      (if
       (then
        (local.set $3
         (i32.add
          (local.get $3)
          (local.get $1)
         )
        )
        (local.set $2
         (i32.add
          (local.get $2)
          (i32.const 1)
         )
        )
        (br $label$3)
       )
      )
     )
     (local.set $1
      (i32.add
       (local.get $1)
       (i32.const 1)
      )
     )
     (br $label$1)
    )
   )
  )
  (local.get $3)
 )
 (export "nested_loop" (func $nested_loop))
)
