(module
 (type $i32_=>_i32 (func (param i32) (result i32)))
 (export "nested_loop" (func $0))
 (func $0 (param $0 i32) (result i32)
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
   (if
    (i32.lt_u
     (local.get $1)
     (local.get $0)
    )
    (block
     (local.set $2
      (i32.const 0)
     )
     (loop $label$3
      (if
       (i32.lt_u
        (local.get $2)
        (local.get $1)
       )
       (block
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
)
