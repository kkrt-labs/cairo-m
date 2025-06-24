use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_collect(c: &mut Criterion) {
    c.bench_function("collect", |b| {
        b.iter(|| black_box((0..1_048_576).collect::<Vec<i32>>()))
    });
}

fn bench_extend(c: &mut Criterion) {
    c.bench_function("extend", |b| {
        b.iter(|| {
            let mut vec = Vec::with_capacity(1_048_576);
            vec.extend(0..1_048_576);
            black_box(vec)
        })
    });
}

fn bench_unsafe(c: &mut Criterion) {
    c.bench_function("unsafe", |b| {
        b.iter(|| {
            let len = 1_048_576;
            let mut vec: Vec<u32> = Vec::with_capacity(len);
            unsafe {
                for i in 0..len {
                    std::ptr::write(vec.as_mut_ptr().add(i), i as u32);
                }
                vec.set_len(len);
            }
            black_box(vec)
        })
    });
}

criterion_group!(benches, bench_collect, bench_extend, bench_unsafe);
criterion_main!(benches);
