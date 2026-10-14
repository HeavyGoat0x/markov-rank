# markov-rank — guide for Claude agents (and for the user working alone)

Read this first. It says **why** this project exists, **how we work**, **what each phase contains
and where it stops**, and **where we are**. Update "Where we are" at the end of every session.

## Why this project exists

Three goals, in this order of priority:

1. **Learn Rust.** The user is learning Rust with this project. They write the code; Claude
   prepares skeletons, reviews, explains and gives hints. This matters more than speed.
2. **University homework (TD).** Course MIN26102 "Modèles stochastiques et simulation"
   (M1 Informatique, UVSQ/Paris-Saclay). TD1 = discrete-time Markov chains; TD2 = implement
   PageRank, test it on sparse matrices from <https://sparse.tamu.edu>, then perturb the graph
   (remove/add links and nodes) and measure the number of iterations when starting from the
   previous stationary distribution.
3. **The real target: a crawler/indexer for the Road To Italy chatbot.** The user's platform
   (Next.js + Supabase, `C:\Users\ramyf\Desktop\italia`) has an AI assistant. This engine will
   crawl Italian university websites and all their sub-pages to build a map of each site, so the
   chatbot "doesn't get lost". PageRank / personalized PageRank on each site's internal link graph
   finds the hub pages (admissions, international students, fees). Relevant Italia pieces:
   `atenei.csv` (~113 universities, no website column), `scripts/fetch-universitaly.mjs`
   (Universitaly/Cineca JSON API), table `program_pages` (official pages, `content_hash` /
   `facts_hash`, conditional GET with ETag/Last-Modified), AI tools in `lib/ai/tools/`
   (see Italia `ARCHITECTURE.md` §8bis). Do not modify the Italia repo from here.

## How we work (important)

- **The user codes, Claude teaches.** Every phase is a skeleton: types, signatures, doc comments
  with hints, `todo!()` bodies, and **complete tests** that fail until the bodies are written.
  Only write an implementation when the user explicitly asks ("fais-le", "rectify", "show me the
  code"), and then explain it.
- **All test suites were validated** against a reference implementation (in a scratch copy, never
  in the repo): with correct code every test passes and `clippy -D warnings` is clean. If a test
  fails, the bug is in the user's code, not in the test.
- **Reviews**: run tests + clippy, list bugs first (failing output, concrete trace), then style.
  Point out tests that pass "by luck".
- The user writes in French or English, often mixed; answer in the language of their message.
  Code comments and doc comments are in **French**; commit messages in **English**.
- Concepts already covered (build on them): ownership/move vs `clone`, borrowing `&`/`&mut`,
  `Option`/`Result`, `?` vs `map_err` vs `match` vs `unwrap`/`expect`, `assert!` semantics,
  iterators (`iter`, `iter_mut`, `enumerate`, `zip`, `collect`), shadowing, scope, closures,
  `impl Display`, unit vs integration vs doc tests, `thiserror` with `#[from]`, `matches!`,
  modules, associated functions (`Type::f`) vs methods (`value.f`), slice `binary_search`.
- Recurring mistakes seen in reviews (watch for them): `assert!` with the inverted condition,
  TypeScript habits (`x!`, `Type name = …`), variables declared inside an `if`/loop and used
  outside, manual index arithmetic with `- 1` on `usize` (underflow), mixing a row offset
  (`row_ptr[i]`) with a row number, swapped indices (`x·A` vs `A·x`), iterating over every
  `(i, j)` instead of the stored entries of a sparse matrix, unclear `expect` messages.

## Layout

```
Cargo.toml          workspace (members = crates/*), shared version/edition, clippy pedantic
clippy.toml         doc-valid-idents (PageRank)
.github/workflows/ci.yml   fmt --check, clippy --workspace --all-targets -D warnings, test --workspace
crates/markov/      DenseMatrix, step, iterate (TD1); sparse.rs: CsrMatrix; parallel.rs (phase 5)
  examples/ex1_2.rs  TD1 exercise 1.2
crates/mtx/         Matrix Market reader -> Coo triplets (0-based), MtxError (thiserror)
crates/pagerank/    lib.rs: Transition, google_step, pagerank (phase 3)
                    perturb.rs + ranking.rs (phase 4), fast.rs + benches/ (phase 5)
crates/cli/         bin `markov-rank`: info / rank / perturb (phase 4)
crates/crawler/     university crawler (phase 6) + bin `crawl`
crates/search/      tokenizer + BM25 inverted index (phase 6)
data/               downloaded matrices and crawl outputs (git-ignored)
```

Design decisions:
- `mtx` does **not** depend on `markov`: it returns COO triplets; `pagerank` glues them.
- Never build the Google matrix G densely: `π·G = α·π·P + (α·Σ_dangling π + 1 − α)·v`, O(nnz).
- `x·A` with CSR is a scatter (`vec_mul`); phase 5 uses `x·A = Aᵀ·x` with a gather on the
  transposed matrix, which parallelises with rayon.
- Library code returns `Result` for anything that can happen; `expect` only for impossible
  cases, with a `# Panics` doc section (clippy pedantic enforces it). No `unwrap` outside tests.
- Randomness always comes from a seeded `StdRng` passed as a parameter (`rand` 0.10:
  `use rand::prelude::*;`, methods on `RngExt`: `random_range`, `random`; `SliceRandom::shuffle`;
  `rand::seq::index::sample`).

## Conventions

- `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test --workspace` must pass before opening a PR. The CI runs exactly these, on PRs
  and on pushes to `main` (pushing a feature branch without a PR does not run the CI).
- Unit tests in `#[cfg(test)] mod tests` next to the code; integration tests in `crates/*/tests/`.
- `proptest` may create `crates/pagerank/proptest-regressions/` when a property test fails:
  delete it if the failure came from a `todo!()`; commit it if it found a real bug.
- Git: PR into `main`, **merge commit** (not squash: each commit counts on the user's GitHub
  graph). Commit messages: Conventional Commits subject (≤72 chars, English) **plus a body**.
  In PowerShell: `git commit -m "subject" -m "body"`.
- Tags: annotated `vX.Y.Z`, one minor version per finished phase; bump `version` in the root
  `Cargo.toml` (then `cargo build` to update `Cargo.lock`) before merging. Never move a pushed tag.
- Remote: <https://github.com/HeavyGoat0x/markov-rank>, default branch `main`.

## Stacked branches (prepared 2026-10-14)

Each phase's skeleton lives on its own branch, each branch built on the previous one:

```
main (v0.2.0) ── phase3-pagerank ── phase4-experiments ── phase5-performance ── phase6-crawler
```

Workflow for each phase N:

1. `git switch phaseN-…` then **`git merge main`** (brings in the finished previous phases;
   skip for phase 3, which starts from `main`).
2. Fill in the `todo!()`s in the order given below; run the phase's tests often.
3. Green tests + clippy + fmt → bump the version → commit → `git push -u origin phaseN-…` →
   PR into `main` → wait for CI → merge (merge commit) → `git switch main && git pull` →
   tag `v0.N.0` and push the tag.

If `git merge main` reports a conflict, it will be in a file both branches touched (usually
`Cargo.lock` or a `pub mod` line): keep both sides, run `cargo build`, then `git add` + `git commit`.

## Phase guide — where each phase starts and stops

### Phase 3 — PageRank · branch `phase3-pagerank` · tag `v0.3.0`
- File: `crates/pagerank/src/lib.rs`. Order: `uniform`, `l1_distance`,
  `Transition::from_adjacency`, `Transition::from_mtx_file`, `google_step`, `pagerank_from`
  (`pagerank` is already written and just calls it).
- Tests: `cargo test -p pagerank` → **22 tests** (19 unit incl. a proptest invariant,
  3 integration in `tests/mtx_file.rs`).
- Stops when all 22 pass. Concepts: `#[from]` error conversion through `?`, the Google matrix
  formula, convergence loop, `Default`, struct update syntax `..Options::default()`.

### Phase 4 — TD2 experiments + CLI · branch `phase4-experiments` · tag `v0.4.0`
- Files: `crates/pagerank/src/ranking.rs` (`top_k`), `crates/pagerank/src/perturb.rs`
  (`Graph::from_mtx_file`, `Graph::transition`, `remove_random_links`, `add_random_links`,
  `add_random_nodes`, `extend_distribution`, `compare_starts`), then `crates/cli/src/main.rs`
  (`cmd_info`, `cmd_rank`, `cmd_perturb`; the clap definitions are already written).
- Tests: `cargo test -p pagerank` (**39** with phase 3) and `cargo test -p markov-rank` (**4**).
- Real data for the TD: on <https://sparse.tamu.edu>, group **SNAP**, matrices `web-Stanford`
  (282k pages, 2.3M links), `web-NotreDame`, `web-BerkStan`, `web-Google` (916k, 5.1M). Download
  the "Matrix Market" `.tar.gz`, then `tar -xzf web-Stanford.tar.gz -C data` (works in
  PowerShell). Run with `cargo run --release -p markov-rank -- rank data/web-Stanford/web-Stanford.mtx`.
- TD deliverable: `perturb` with `--csv`, several `--runs`, different sizes of change; plot
  uniform vs warm iterations. Observed on a small test graph: warm start saves ~15 % of the
  iterations; the gain is bigger when the change is small relative to the graph.
- Stops when the tests pass and `rank` / `perturb` work on a real SNAP matrix.

### Phase 5 — Performance · branch `phase5-performance` · tag `v0.5.0`
- Files: `crates/markov/src/parallel.rs` (`mul_vec`, `par_mul_vec`),
  `crates/pagerank/src/fast.rs` (`From<&Transition> for TransposedTransition`,
  `google_step_parallel`, `pagerank_parallel`, `top_k_fast`). Benchmarks already written in
  `crates/pagerank/benches/pagerank.rs`.
- Tests: `cargo test -p markov` (**32**), `cargo test -p pagerank` (**44**).
- Measure: `cargo bench -p pagerank` (report in `target/criterion/report/index.html`). Reference
  numbers on the user's PC: on 200k pages / 2M links, scatter ≈ gather ≈ 15 ms, parallel gather
  ≈ 5.5 ms (~3×); top 10 of 1M scores: full sort ≈ 190 ms, selection ≈ 23 ms (~8×).
- Optional bonus: refactor the convergence loop shared by `pagerank_from` and
  `pagerank_parallel` into a private function taking the step as a closure; Gauss-Seidel.
- Stops when tests pass and benchmarks show the parallel version faster.

### Phase 6 — University crawler + search · branch `phase6-crawler` · tag `v0.6.0`
- Crate `crawler`, in this order (pure modules first, network last):
  `html.rs::collapse_whitespace` → `urls.rs` → `sitemap.rs` → `robots.rs` → `frontier.rs` →
  `graph.rs` → `html.rs` (rest) → `export.rs` → `crawl.rs` (`fetch_html`, `fetch_robots`,
  `crawl`, `build_records`) → `src/bin/crawl.rs` (`run`).
- Crate `search`: `fold_accents`, `tokenize`, `add_document`, `document_frequency`, `idf`,
  `search`, `rerank_with_pagerank`.
- Tests: `cargo test -p crawler` (**34**: 31 unit, 2 bin, 1 end-to-end test that serves a small
  site on 127.0.0.1 — no internet needed), `cargo test -p search` (**11**).
  Whole workspace with everything done: **144 tests**.
- First real run, small and polite:
  `cargo run --release -p crawler --bin crawl -- https://www.unimi.it/ --max-pages 100 --out data/unimi.jsonl`.
  Never lower the 1 s delay on a real site; respect robots.txt (the code does).
- Stops when tests pass and a 100-page crawl of one university produces a sensible JSONL
  (home and admission pages near the top).

### After phase 6 (not prepared yet — design with Claude)
- Seeds for all universities (`atenei.csv` has no website column: get domains from the
  Universitaly API or build the list once), crawl them one by one, conditional GET with
  ETag/Last-Modified + `content_hash` to re-crawl only what changed (same idea as Italia's
  `program_pages`).
- Personalized PageRank seeded on "international students / admissions" pages
  (`pagerank_from` with a `teleport` vector).
- Italia integration: a Node script imports the JSONL into a Supabase table (e.g. `site_pages`),
  and an AI tool lets the chatbot search a university's pages (BM25 + PageRank).

## Where we are

_Last update: 2026-10-14._

- `main` = **v0.2.0** (phases 0–2 done: workspace + CI, TD1 dense Markov chains, `mtx` reader,
  `CsrMatrix`). 47 tests green on `main`.
- Skeletons for phases 3, 4, 5 and 6 are committed on their stacked branches (local only, not
  pushed yet). **Next: phase 3 on `phase3-pagerank`.**
- Progress estimate (weights: setup 5, P1 10, P2 15, P3 15, P4 10, P5 10, P6 35): ~30 % of the
  whole project, ~55 % of the TD part (phases 0–4).
