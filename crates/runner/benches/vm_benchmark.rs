use std::time::Duration;

use cairo_m_common::{Instruction, Program};
use cairo_m_runner::vm::VM;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use tempfile::NamedTempFile;

const FIB_N: u32 = 1_000_000;
const BENCHMARK_DURATION_SECS: u64 = 30;

/// Creates a Fibonacci program for the given n value.
/// Returns the instructions
pub fn create_fib_program(n: u32) -> Vec<Instruction> {
    let instructions = vec![
        // Setup
        Instruction::try_from([6, n, 0, 0]).unwrap(), // store_imm: [fp+0] = counter
        Instruction::try_from([6, 0, 0, 1]).unwrap(), // store_imm: [fp+1] = a = F_0 = 0
        Instruction::try_from([6, 1, 0, 2]).unwrap(), // store_imm: [fp+2] = b = F_1 = 1
        // Loop condition check
        // while counter != 0 jump to loop body
        Instruction::try_from([31, 0, 2, 0]).unwrap(), // jnz_fp_imm: jmp rel 2 if [fp + 0] != 0  (pc=3 here, pc=5 in beginning of loop body)
        // Exit jump if counter was 0
        Instruction::try_from([20, 10, 0, 0]).unwrap(), // jmp_abs_imm: jmp abs 10
        // Loop body
        Instruction::try_from([4, 1, 0, 3]).unwrap(), // store_deref_fp: [fp+3] = [fp+1] (tmp = a)
        Instruction::try_from([4, 2, 0, 1]).unwrap(), // store_deref_fp: [fp+1] = [fp+2] (a = b)
        Instruction::try_from([0, 3, 2, 2]).unwrap(), // store_add_fp_fp: [fp+2] = [fp+3] + [fp+2] (b = temp + b)
        Instruction::try_from([3, 0, 1, 0]).unwrap(), // store_sub_fp_imm: [fp+0] = [fp+0] - 1 (counter--)
        // Jump back to condition check
        Instruction::try_from([20, 3, 0, 0]).unwrap(), // jmp_abs_imm: jmp abs 3
    ];

    instructions
}

fn fibonacci_1m_benchmark(c: &mut Criterion) {
    // Create Fibonacci(1M) program
    let instructions = create_fib_program(FIB_N);
    let program = Program::from(instructions);

    // Run once to get metrics for throughput calculation and reuse for serialization benchmarks
    let mut vm = VM::try_from(&program).unwrap();
    vm.run_from_entrypoint(0, 3).unwrap();
    let mut group = c.benchmark_group("fibonacci_1m");
    group.throughput(Throughput::Elements(vm.trace.len() as u64));
    group.measurement_time(Duration::from_secs(BENCHMARK_DURATION_SECS));

    group.bench_function("e2e", |b| {
        b.iter(|| {
            let mut vm = VM::try_from(&program).unwrap();

            let trace_file = NamedTempFile::new().expect("Failed to create trace temp file");
            let memory_trace_file =
                NamedTempFile::new().expect("Failed to create memory trace temp file");

            vm.run_from_entrypoint(0, 3).unwrap();
            vm.write_binary_trace(trace_file.path()).unwrap();
            vm.write_binary_memory_trace(memory_trace_file.path())
                .unwrap();

            black_box(vm)
        })
    });

    group.bench_function("execution_only", |b| {
        b.iter(|| {
            let mut vm = VM::try_from(&program).unwrap();
            vm.run_from_entrypoint(0, 3).unwrap();
            black_box(vm)
        })
    });

    group.bench_function("io_only", |b| {
        // Pre-execute the VM for I/O testing
        let mut vm = VM::try_from(&program).unwrap();
        vm.run_from_entrypoint(0, 3).unwrap();

        let trace_file = NamedTempFile::new().expect("Failed to create trace temp file");
        let memory_trace_file =
            NamedTempFile::new().expect("Failed to create memory trace temp file");

        b.iter(|| {
            vm.write_binary_trace(trace_file.path()).unwrap();
            vm.write_binary_memory_trace(memory_trace_file.path())
                .unwrap();
            black_box(())
        })
    });

    group.bench_function("serialize_vm_trace", |b| {
        b.iter(|| {
            let serialized = vm.serialize_trace();
            black_box(serialized)
        })
    });

    group.bench_function("serialize_memory_trace", |b| {
        b.iter(|| {
            let serialized = vm.memory.serialize_trace();
            black_box(serialized)
        })
    });

    group.finish();
}

criterion_group!(benches, fibonacci_1m_benchmark);
criterion_main!(benches);
