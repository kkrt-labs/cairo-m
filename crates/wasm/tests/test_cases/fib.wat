(module $fib-9247652854a11427.wasm
 (type $i32_=>_i32 (func (param i32) (result i32)))
 (memory $0 16)
 (global $__stack_pointer (mut i32) (i32.const 1048576))
 (global $global$1 i32 (i32.const 1048576))
 (global $global$2 i32 (i32.const 1048576))
 (export "memory" (memory $0))
 (export "fib" (func $fib))
 (export "__data_end" (global $global$1))
 (export "__heap_base" (global $global$2))
 (func $fib (param $0 i32) (result i32)
  (local $1 i32)
  (local $2 i32)
  (local $3 i32)
  (block $label$1
   (br_if $label$1
    (local.get $0)
   )
   (return
    (i32.const 0)
   )
  )
  (local.set $1
   (i32.and
    (local.get $0)
    (i32.const 7)
   )
  )
  (block $label$2
   (block $label$3
    (br_if $label$3
     (i32.ge_u
      (local.get $0)
      (i32.const 8)
     )
    )
    (local.set $0
     (i32.const 1)
    )
    (local.set $2
     (i32.const 0)
    )
    (br $label$2)
   )
   (local.set $3
    (i32.and
     (local.get $0)
     (i32.const -8)
    )
   )
   (local.set $0
    (i32.const 1)
   )
   (local.set $2
    (i32.const 0)
   )
   (loop $label$4
    (local.set $0
     (i32.add
      (local.tee $2
       (i32.add
        (local.tee $0
         (i32.add
          (local.tee $2
           (i32.add
            (local.tee $0
             (i32.add
              (local.tee $2
               (i32.add
                (local.tee $0
                 (i32.add
                  (local.tee $2
                   (i32.add
                    (local.get $0)
                    (local.get $2)
                   )
                  )
                  (local.get $0)
                 )
                )
                (local.get $2)
               )
              )
              (local.get $0)
             )
            )
            (local.get $2)
           )
          )
          (local.get $0)
         )
        )
        (local.get $2)
       )
      )
      (local.get $0)
     )
    )
    (br_if $label$4
     (local.tee $3
      (i32.add
       (local.get $3)
       (i32.const -8)
      )
     )
    )
   )
  )
  (block $label$5
   (br_if $label$5
    (i32.eqz
     (local.get $1)
    )
   )
   (local.set $3
    (local.get $2)
   )
   (loop $label$6
    (local.set $0
     (i32.add
      (local.tee $2
       (local.get $0)
      )
      (local.get $3)
     )
    )
    (local.set $3
     (local.get $2)
    )
    (br_if $label$6
     (local.tee $1
      (i32.add
       (local.get $1)
       (i32.const -1)
      )
     )
    )
   )
  )
  (local.get $2)
 )
 ;; custom section "producers", size 59
)
