(module
  (type (;0;) (func (param i32) (result i32)))
  (func $simple_if (type 0) (param $x i32) (result i32)
    ;; Simple if statement: if x > 5, return x + 10, else return x
    local.get $x
    i32.const 5
    i32.gt_u
    
    if (result i32)
      ;; If x > 5, return x + 10
      local.get $x
      i32.const 10
      i32.add
    else
      ;; If x <= 5, return x as is
      local.get $x
    end
  )
  
  (export "simple_if" (func $simple_if))
)
