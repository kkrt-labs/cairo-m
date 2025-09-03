#![no_main]
#![no_std]

extern crate panic_abort;

#[unsafe(no_mangle)]
fn array_sum(x: [u32; 10]) -> u32 {
    return x.into_iter().sum();
}
