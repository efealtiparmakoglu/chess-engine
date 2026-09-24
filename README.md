# chess-engine

Chess move generation **verified by perft**, an alpha-beta search
engine, SAN/PGN output and a material-evolution trace — Rust core plus
an offline TypeScript game viewer.

```text
$ chess-engine perft -d 4
perft(1) =        20  (0.00s)
perft(2) =       400  (0.00s)
perft(3) =      8902  (0.00s)
perft(4) =    197281  (0.01s)

$ chess-engine play -d 3 --plies 60
self-play done: 60 moves -> chess.pgn, game.json, evolution.csv
```

## The engine (Rust, zero crates)

- **Move generation** — full legality: pins, checks, castling (with
  attacked-square tests), en passant, promotions. Correctness is pinned
  by perft: startpos to depth 4, the Kiwipete position to depth 3
  (48 / 2 039 / 97 862), the en-passant position (14 / 191 / 2 812 /
  43 238) and the promotion position — all matching the published
  values exactly.
- **Search** — negamax with alpha-beta, capture-first move ordering and
  a depth-bounded quiescence search (unbounded capture chains were the
  first bug the perft/perf harness caught).
- **Evaluation** — material plus light piece-square preferences.
- **PGN + evolution** — self-play writes `chess.pgn` (SAN with
  disambiguation, `x`, `=Q`, `+`/`#`, `O-O`/`O-O-O`, headers) and
  `game.json` / `evolution.csv` with the material totals after every
  ply.

## The viewer (TypeScript, single file offline)

`npm run build:web` inlines the stripped TS into
`web/dist/index.html`: paste `game.json`, step through the game
(⏮ ◀ ▶ ⏭ or the slider) and watch the material-evolution chart —
blunders appear as cliffs.

## Development

```console
$ cargo test --release        # perft suite, SAN, en passant, search
$ cargo clippy --all-targets -- -D warnings
$ npm run build:web
```

## License

MIT — see [LICENSE](LICENSE).
