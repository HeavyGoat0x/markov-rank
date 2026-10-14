# markov-rank — guide for Claude agents

Read this first. It says **why** this project exists, **how we work**, and **where we are**.
Update the "Where we are" section at the end of every work session.

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

- **The user codes, Claude teaches.** For each phase, Claude writes a skeleton: types,
  signatures, doc comments with hints, `todo!()` bodies, and **complete tests** that fail.
  The user fills in the bodies. Only write the implementation when the user explicitly asks
  ("fais-le", "rectify", "show me the code"), and then explain it line by line.
- **Verify every test suite before handing it over**: copy the workspace to the scratchpad,
  fill it with a reference implementation, run `cargo test` + `cargo clippy -- -D warnings`.
  Never touch the user's files for that.
- **Reviews**: run tests + clippy, then list bugs first (with the failing test output and a
  concrete trace), then style. Point out tests that pass "by luck".
- The user writes in French or English, often mixed; answer in the language of their message.
  Code comments and doc comments are in **French**; commit messages in **English**.
- Concepts already covered (build on them, don't re-explain from scratch): ownership/move vs
  `clone`, borrowing `&`/`&mut`, `Option`/`Result`, `?` vs `map_err` vs `match` vs
  `unwrap`/`expect`, `assert!` semantics, iterators (`iter`, `iter_mut`, `enumerate`, `zip`,
  `collect`), shadowing, scope, closures, `impl Display`, unit vs integration vs doc tests,
  `thiserror` with `#[from]`, `matches!`.

## Layout

```
Cargo.toml          workspace (members = crates/*), shared version/edition, clippy pedantic
clippy.toml         doc-valid-idents (PageRank)
.github/workflows/ci.yml   fmt --check, clippy --workspace --all-targets -D warnings, test --workspace
crates/markov/      lib: DenseMatrix, step, iterate (TD1) + sparse.rs: CsrMatrix
  examples/ex1_2.rs  TD1 exercise 1.2 (cargo run -p markov --example ex1_2)
crates/mtx/         lib: Matrix Market reader -> Coo triplets (0-based), MtxError (thiserror)
  tests/            integration tests + tests/data/*.mtx
crates/pagerank/    lib: placeholder (phase 3)
crates/cli/         bin `markov-rank`: placeholder (phase 4)
```

Design decisions:
- `mtx` does **not** depend on `markov`: it returns COO triplets; `pagerank` glues them.
- Never build the Google matrix G densely. Use `π·G = α·π·P + (α·(π·d) + 1 − α)/n · e`
  (d = dangling nodes) with P in CSR: O(nnz) per iteration.
- `x · A` with CSR is a scatter over rows (`vec_mul`); a transposed/gather version comes in the
  optimisation phase (parallelism with rayon).
- Library code returns `Result` for anything that can happen; `expect` only for impossible
  cases, with a `# Panics` doc section (clippy pedantic enforces it). No `unwrap` outside tests.

## Conventions

- `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test --workspace` must pass before pushing. The CI runs exactly these.
- Unit tests in `#[cfg(test)] mod tests` next to the code; integration tests in `crates/*/tests/`.
- Git: one branch per phase from `main`, PR into `main`, **merge commit** (not squash: each
  commit counts on the user's GitHub graph). Never push a branch while `todo!()` remain
  (CI would be red); commit locally instead.
- Commit messages: Conventional Commits subject (outcome-focused, ≤72 chars, English) **plus a
  body** (what, why, results, how to run). In PowerShell: `git commit -m "subject" -m "body"`.
- Tags: annotated `vX.Y.Z`, one minor version per finished phase; bump `version` in the root
  `Cargo.toml` to match. `phase1` is an old tag, kept as is. Never move a pushed tag.
- Remote: <https://github.com/HeavyGoat0x/markov-rank>, default branch `main`.

## Roadmap

| Phase | Content | Status |
|---|---|---|
| 0 | Workspace, git, CI (GitHub Actions) | ✅ done |
| 1 | TD1: dense matrix, π·P, iteration, P^1024, example ex1_2 | ✅ done — tag `v0.1.0` |
| 2 | Sparse: `mtx` reader + `CsrMatrix` (from_triplets, row, get, vec_mul, conversions, transpose) | ✅ code done — tag `v0.2.0` pending |
| 3 | PageRank: row normalisation, dangling nodes, implicit G, L1 convergence, iteration count; tests vs dense G on small graphs, proptest invariants | ⬜ |
| 4 | TD2 experiments: `clap` CLI, real sparse.tamu.edu matrices (SNAP web graphs), seeded random perturbations, warm start vs uniform start, CSV + plots | ⬜ |
| 5 | Optimisation: criterion benchmarks, rayon, transposed CSR (gather), Gauss-Seidel, profiling (samply) | ⬜ |
| 6 | Crawler for Italian university sites: seeds per university, polite crawling (robots.txt, sitemap.xml first, rate limit, User-Agent with contact), URL normalisation, conditional GET + content hash, per-site link graph, PageRank + personalized PageRank, BM25 index (it/en), JSONL export consumed by an Italia script (later sqlx → Supabase). Crates: tokio, reqwest, scraper, url, texting_robots | ⬜ |

## Where we are

_Last update: 2026-10-14._

- **Phase 2 code complete** on branch `phase2-sparse`: 47 tests green (markov 28, mtx 17 + 2
  integration), clippy `-D warnings` clean. Remaining: commit sparse.rs, bump version to 0.2.0,
  push, PR, merge (merge commit), tag `v0.2.0`.
- `mtx`: parse_header, parse_size, parse_entry, read, read_file.
- `markov/src/sparse.rs`: `CsrMatrix` with from_triplets, from_dense, to_dense, row, get
  (slice `binary_search`), transpose, vec_mul (scatter over `row(i)`, O(nnz)).
- Recurring mistakes seen in reviews (watch for them): `assert!` with the inverted condition,
  TypeScript habits (`x!`, `Type name = …`), variables declared inside an `if`/loop and used
  outside, manual index arithmetic with `- 1` on `usize` (underflow), mixing a row offset
  (`row_ptr[i]`) with a row number, unclear `expect` messages.
- Next steps: finish sparse.rs → commit → push → PR → merge → tag `v0.2.0` (bump Cargo.toml
  version) → phase 3 skeleton (pagerank crate) with tests, same method as phases 1–2.
