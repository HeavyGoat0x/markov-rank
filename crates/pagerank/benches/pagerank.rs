//! Benchmarks de la phase 5 : `cargo bench -p pagerank`.
//!
//! Criterion répète chaque mesure, calcule moyenne et dispersion, et compare avec la dernière
//! exécution (rapport HTML dans `target/criterion/report/index.html`). Toujours mesurer en
//! `--release` (c'est le cas de `cargo bench`) et sur une machine au repos.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use markov::CsrMatrix;
use pagerank::fast::{TransposedTransition, google_step_parallel, pagerank_parallel, top_k_fast};
use pagerank::ranking::top_k;
use pagerank::{Options, Transition, google_step, pagerank, uniform};
use rand::prelude::*;

/// Graphe aléatoire reproductible : `n` pages, `links` liens sortants chacune.
fn random_web(n: usize, links: usize) -> Transition {
    let mut rng = StdRng::seed_from_u64(2026);
    let mut triplets = Vec::with_capacity(n * links);
    for i in 0..n {
        for _ in 0..links {
            triplets.push((i, rng.random_range(0..n), 1.0));
        }
    }
    let adjacency = CsrMatrix::from_triplets(n, n, triplets).expect("indices valides");
    Transition::from_adjacency(&adjacency).expect("graphe carré non vide")
}

fn products(c: &mut Criterion) {
    let t = random_web(200_000, 10);
    let pt = t.p.transpose();
    let x = uniform(t.n());
    let mut group = c.benchmark_group("produit (200k pages, 2M liens)");
    group.bench_function("vec_mul (scatter)", |b| {
        b.iter(|| t.p.vec_mul(black_box(&x)));
    });
    group.bench_function("mul_vec sur Pᵀ (gather)", |b| {
        b.iter(|| pt.mul_vec(black_box(&x)));
    });
    group.bench_function("par_mul_vec sur Pᵀ (gather parallèle)", |b| {
        b.iter(|| pt.par_mul_vec(black_box(&x)));
    });
    group.finish();
}

fn steps(c: &mut Criterion) {
    let t = random_web(200_000, 10);
    let tt = TransposedTransition::from(&t);
    let u = uniform(t.n());
    let mut group = c.benchmark_group("un pas de PageRank");
    group.bench_function("google_step", |b| {
        b.iter(|| google_step(black_box(&u), &t, 0.85, &u));
    });
    group.bench_function("google_step_parallel", |b| {
        b.iter(|| google_step_parallel(black_box(&u), &tt, 0.85, &u));
    });
    group.finish();
}

fn full(c: &mut Criterion) {
    let t = random_web(50_000, 10);
    let opts = Options::default();
    let mut group = c.benchmark_group("PageRank complet (50k pages)");
    group.sample_size(10); // chaque mesure est longue : 10 échantillons suffisent
    group.bench_function("pagerank", |b| b.iter(|| pagerank(black_box(&t), &opts)));
    group.bench_function("pagerank_parallel", |b| {
        b.iter(|| pagerank_parallel(black_box(&t), &opts));
    });
    group.finish();
}

fn ranking(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(7);
    let scores: Vec<f64> = (0..1_000_000).map(|_| rng.random()).collect();
    let mut group = c.benchmark_group("top 10 sur 1M de scores");
    group.bench_function("top_k (tri complet)", |b| {
        b.iter(|| top_k(black_box(&scores), 10));
    });
    group.bench_function("top_k_fast (sélection)", |b| {
        b.iter(|| top_k_fast(black_box(&scores), 10));
    });
    group.finish();
}

criterion_group!(benches, products, steps, full, ranking);
criterion_main!(benches);
