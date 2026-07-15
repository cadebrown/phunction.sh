//! Engine throughput benchmarks.
//!
//! The number that matters: one 128-frame block must render in well under
//! 2.67ms (the real-time budget at 48kHz) on the *slowest* target device —
//! a phone running wasm. Native numbers here are the canary, not the proof;
//! rule of thumb: keep native ≥ 50× real-time so wasm-on-a-phone still
//! clears 5×.

use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use phazor_core::{Command, Engine, ParamId, Step, QUANTUM};
use std::hint::black_box;

fn full_polyphony_engine() -> Engine {
    let mut e = Engine::new(48_000.0);
    e.apply(Command::SetParam {
        id: ParamId::OscBrightness,
        value: 1.0,
    });
    for n in 0..16u8 {
        e.apply(Command::NoteOn {
            note: 36 + n * 2,
            vel: 100,
        });
    }
    for i in 0..16u8 {
        e.apply(Command::SetStep {
            index: i,
            step: Some(Step {
                note: 40 + (i % 12),
                vel: 100,
                gate: 0.8,
            }),
        });
    }
    e.apply(Command::Play);
    e
}

fn bench_process(c: &mut Criterion) {
    let mut g = c.benchmark_group("engine");
    g.throughput(Throughput::Elements(QUANTUM as u64));

    g.bench_function("block_16_voices_seq_running", |b| {
        b.iter_batched_ref(
            full_polyphony_engine,
            |e| {
                let mut l = [0.0f32; QUANTUM];
                let mut r = [0.0f32; QUANTUM];
                for _ in 0..64 {
                    black_box(e.process(&mut l, &mut r));
                }
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("block_idle", |b| {
        b.iter_batched_ref(
            || Engine::new(48_000.0),
            |e| {
                let mut l = [0.0f32; QUANTUM];
                let mut r = [0.0f32; QUANTUM];
                for _ in 0..64 {
                    black_box(e.process(&mut l, &mut r));
                }
            },
            BatchSize::SmallInput,
        );
    });

    g.finish();
}

criterion_group!(benches, bench_process);
criterion_main!(benches);
