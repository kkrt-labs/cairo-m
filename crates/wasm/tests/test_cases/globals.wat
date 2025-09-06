(module
  ;; Mutable globals
  (global $g1 (mut i32) (i32.const 0))
  (global $g2 (mut i32) (i32.const 0))


;; Store two values in globals and add them
;; This checks that the get and set work as expected and that there are no collisions between the globals
(func $main (param $a i32) (param $b i32) (result i32)
  local.get $a
  global.set $g1
  local.get $b
  global.set $g2
  global.get $g1
  global.get $g2
  i32.add
)

(memory 1)
(export "memory" (memory 0))
(export "main" (func $main))
)
