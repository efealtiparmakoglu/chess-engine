//! CLI: perft verification, best-move search, self-play PGN output.

use chess_engine_lib::board::{Board, START_FEN};
use chess_engine_lib::engine;
use std::process::exit;

const USAGE: &str = r#"chess-engine v1.0.0 — move generation verified by perft, alpha-beta search

USAGE
  chess-engine perft [-d N] [--fen FEN]     count leaf nodes (perft suite)
  chess-engine divide [-d N] [--fen FEN]    perft split by root move
  chess-engine bestmove [-d N] [--fen FEN]  search the best move, print SAN
  chess-engine play [-d N] [--plies N]      engine-vs-engine self-play;
                                            writes chess.pgn + evolution.csv
  chess-engine (-h | --help | -V | --version)

Exit codes: 0 ok, 1 failure, 2 usage error."#;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut depth = 3usize;
    let mut plies = 120usize;
    let mut fen: Option<String> = None;
    let mut command: Option<String> = None;
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "-d" | "--depth" => {
                depth = argv.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or_else(|| {
                    fail_usage("-d requires a number")
                });
                i += 1;
            }
            "--plies" => {
                plies = argv.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or_else(|| {
                    fail_usage("--plies requires a number")
                });
                i += 1;
            }
            "--fen" => {
                fen = Some(
                    argv.get(i + 1).cloned().unwrap_or_else(|| fail_usage("--fen requires a value")),
                );
                i += 1;
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return;
            }
            "-V" | "--version" => {
                println!("chess-engine 1.0.0");
                return;
            }
            other => {
                if command.is_some() {
                    fail_usage(&format!("unexpected argument \"{other}\""));
                }
                command = Some(other.to_string());
            }
        }
        i += 1;
    }
    let Some(command) = command else {
        println!("{USAGE}");
        return;
    };

    let board = match &fen {
        Some(fen) => Board::from_fen(fen).unwrap_or_else(|e| fail_runtime(e.to_string())),
        None => Board::startpos(),
    };

    match command.as_str() {
        "perft" => {
            for d in 1..=depth {
                let start = std::time::Instant::now();
                let count = engine::perft(&board, d);
                println!(
                    "perft({d}) = {count:>9}  ({:.2}s)",
                    start.elapsed().as_secs_f64()
                );
            }
        }
        "divide" => {
            for (san, count) in engine::divide(&board, depth) {
                println!("{san:>8} {count}");
            }
        }
        "bestmove" => {
            let start = std::time::Instant::now();
            match engine::best_move(&board, depth) {
                Some(mv) => {
                    let san = engine::to_san(&board, &mv);
                    println!("bestmove {san} ({:.2}s)", start.elapsed().as_secs_f64());
                }
                None => fail_runtime("no legal moves in this position".into()),
            }
        }
        "play" => {
            let game = engine::self_play(&board, depth, plies);
            let pgn = build_pgn(&board, &game);
            std::fs::write("chess.pgn", &pgn)
                .unwrap_or_else(|e| fail_runtime(format!("cannot write chess.pgn: {e}")));
            let mut csv = String::from("ply,white,black\n");
            for (ply, (white, black)) in game.material.iter().enumerate() {
                csv.push_str(&format!("{ply},{white},{black}\n"));
            }
            std::fs::write("evolution.csv", &csv)
                .unwrap_or_else(|e| fail_runtime(format!("cannot write evolution.csv: {e}")));
            let json = format!(
                "{{\"sans\":{},\"fens\":{},\"material\":{}}}",
                json_string_array(&game.sans),
                json_string_array(&game.fens),
                format!(
                    "[{}]",
                    game.material
                        .iter()
                        .map(|(w, b)| format!("[{w},{b}]"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            );
            std::fs::write("game.json", &json)
                .unwrap_or_else(|e| fail_runtime(format!("cannot write game.json: {e}")));
            println!(
                "self-play done: {} moves, {} plies -> chess.pgn, game.json, evolution.csv",
                game.sans.len(),
                game.material.len() - 1
            );
            println!("{}", game.sans.join(" "));
        }
        other => fail_usage(&format!("unknown command \"{other}\"")),
    }
}

fn build_pgn(start: &Board, game: &engine::SelfPlayGame) -> String {
    let result = "*"; // self-play results are not adjudicated
    let mut pgn = String::new();
    pgn.push_str(&format!(
        "[Event \"chess-engine self-play\"]\n[Site \"local\"]\n[Date \"{}\"]\n",
        time_date()
    ));
    pgn.push_str(&format!(
        "[White \"chess-engine d3\"]\n[Black \"chess-engine d3\"]\n[Result \"{result}\"]\n"
    ));
    if start.to_fen() != START_FEN {
        pgn.push_str(&format!("[FEN \"{}\"]\n[SetUp \"1\"]\n", start.to_fen()));
    }
    pgn.push('\n');
    let mut line = String::new();
    for (i, san) in game.sans.iter().enumerate() {
        if i % 2 == 0 {
            line.push_str(&format!("{}. ", i / 2 + 1));
        }
        line.push_str(san);
        line.push(' ');
        if line.len() > 72 {
            pgn.push_str(line.trim_end());
            pgn.push('\n');
            line.clear();
        }
    }
    pgn.push_str(line.trim_end());
    pgn.push_str(&format!(" {result}\n"));
    pgn
}

fn time_date() -> String {
    // std has no date formatting; a UTC timestamp is enough for a header
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("UTC {secs}")
}

fn json_string_array(items: &[String]) -> String {
    let quoted: Vec<String> = items
        .iter()
        .map(|item| {
            let escaped = item.replace('\\', "\\\\").replace('"', "\\\"");
            format!("\"{escaped}\"")
        })
        .collect();
    format!("[{}]", quoted.join(","))
}

fn fail_usage(message: &str) -> ! {
    eprintln!("chess-engine: {message}");
    eprintln!("chess-engine: run 'chess-engine --help' for usage");
    exit(2);
}

fn fail_runtime(message: String) -> ! {
    eprintln!("chess-engine: {message}");
    exit(1);
}
