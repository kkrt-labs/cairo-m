(module
  (type (;0;) (func (result i32)))
  (func $main (type 0) (result i32)
    (local $counter i32)
    (local $sum i32)
    
    ;; Initialize counter to 0
    i32.const 0
    local.set $counter
    
    ;; Initialize sum to 0
    i32.const 0
    local.set $sum
    
    ;; Loop: while counter < 3
    loop $loop
      ;; Add counter to sum
      local.get $sum
      local.get $counter
      i32.add
      local.set $sum
      
      ;; Increment counter
      local.get $counter
      i32.const 1
      i32.add
      local.set $counter
      
      ;; Check if counter < 3, continue loop if true
      local.get $counter
      i32.const 3
      i32.ne
      br_if $loop
    end
    
    ;; Return sum (should be 0+1+2 = 3)
    local.get $sum
  )
  (memory (;0;) 1)
  (export "main" (func $main))
  (export "memory" (memory 0))
) 
