/* DOM layer: replays a chess-engine self-play game (game.json content),
 * renders the board from FEN positions and draws the material-evolution
 * chart as SVG. Paste the game.json text, step through the moves. */

type Kind = "p" | "n" | "b" | "r" | "q" | "k";
type Piece = { color: "w" | "b"; kind: Kind };

interface GameJson {
  sans: string[];
  fens: string[];
  material: [number, number][];
}

const GLYPH: Record<string, string> = {
  wk: "♔", wq: "♕", wr: "♖", wb: "♗", wn: "♘", wp: "♙",
  bk: "♚", bq: "♛", br: "♜", bb: "♝", bn: "♞", bp: "♟",
};

let fens: string[] = [];
let sans: string[] = [];
let material: [number, number][] = [];
let ply = 0;

function parseFen(fen: string): (Piece | null)[] {
  const squares: (Piece | null)[] = new Array(64).fill(null);
  const boardPart = fen.split(" ")[0] as string;
  let rank = 7;
  let file = 0;
  for (const ch of boardPart) {
    if (ch === "/") {
      rank--;
      file = 0;
    } else if (/\d/.test(ch)) {
      file += Number(ch);
    } else {
      const color = ch === ch.toUpperCase() ? "w" : "b";
      squares[rank * 8 + file] = { color, kind: ch.toLowerCase() as Kind };
      file++;
    }
  }
  return squares;
}

function paintBoard(fen: string): void {
  const squares = parseFen(fen);
  const boardEl = document.getElementById("board") as HTMLElement;
  boardEl.innerHTML = "";
  for (let rank = 7; rank >= 0; rank--) {
    for (let file = 0; file < 8; file++) {
      const cellEl = document.createElement("div");
      const piece = squares[rank * 8 + file];
      cellEl.className =
        "cell " + ((rank + file) % 2 === 0 ? "light" : "dark");
      if (piece) {
        cellEl.textContent = GLYPH[piece.color + piece.kind] ?? "?";
      }
      boardEl.appendChild(cellEl);
    }
  }
}

function paintMaterial(): void {
  const chart = document.getElementById("chart") as HTMLElement;
  const width = 460;
  const height = 120;
  if (material.length < 2) {
    chart.innerHTML = "";
    return;
  }
  const maxPly = material.length - 1;
  const values = material.flatMap((pair) => [pair[0], pair[1]]);
  const maxValue = Math.max(...values) * 1.05;
  const toX = (ply: number) => 30 + (ply / maxPly) * (width - 40);
  const toY = (value: number) => height - 24 - (value / maxValue) * (height - 40);
  const line = (index: 0 | 1, color: string) => {
    const points = material
      .map((pair, plyIdx) => `${toX(plyIdx).toFixed(1)},${toY(pair[index]).toFixed(1)}`)
      .join(" ");
    return `<polyline points="${points}" fill="none" stroke="${color}" stroke-width="1.5"/>`;
  };
  let svg =
    `<svg width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">` +
    `<rect width="100%" height="100%" fill="#fffdf7"/>` +
    `<line x1="30" y1="${height - 24}" x2="${width - 10}" y2="${height - 24}" stroke="#999"/>` +
    line(0, "#2e7d32") +
    line(1, "#8a5a2b");
  for (let step = 0; step <= maxPly; step += Math.max(1, Math.floor(maxPly / 8))) {
    svg += `<text x="${toX(step)}" y="${height - 8}" font-size="9" text-anchor="middle" fill="#666">${step}</text>`;
  }
  svg += `<text x="4" y="14" font-size="10" fill="#2e7d32">white</text>`;
  svg += `<text x="4" y="28" font-size="10" fill="#8a5a2b">black</text>`;
  svg += `</svg>`;
  chart.innerHTML = svg;
}

function showPly(): void {
  const fen = fens[ply];
  if (fen) paintBoard(fen);
  const status = document.getElementById("status") as HTMLElement;
  const moveNumber = Math.floor(ply / 2) + 1;
  const moveText = ply === 0 ? "start" : `${moveNumber}${ply % 2 === 1 ? "." : "..."} ${sans[ply - 1] ?? ""}`;
  status.textContent = `ply ${ply}/${Math.max(fens.length - 1, 0)} — ${moveText}`;
  const pair = material[ply];
  if (pair) {
    status.textContent += ` · material ${pair[0]} / ${pair[1]}`;
  }
  paintMaterial();
  const slider = document.getElementById("ply") as HTMLInputElement;
  slider.max = String(Math.max(fens.length - 1, 0));
  slider.value = String(ply);
}

function loadGame(text: string): boolean {
  try {
    const parsed = JSON.parse(text) as GameJson;
    if (!Array.isArray(parsed.fens) || parsed.fens.length < 1) return false;
    fens = parsed.fens;
    sans = parsed.sans ?? [];
    material = parsed.material ?? [];
    ply = 0;
    showPly();
    return true;
  } catch {
    return false;
  }
}

function setup(): void {
  const saved = localStorage.getItem("chess-last-game");
  const input = document.getElementById("game-json") as HTMLTextAreaElement;
  if (saved) input.value = saved;
  if (input.value.trim() !== "" && loadGame(input.value)) {
    /* restored the previous session */
  } else {
    // start from the initial position so the board is never empty
    loadGame(JSON.stringify({
      sans: [],
      fens: ["rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"],
      material: [[4000, 4000]],
    }));
  }
  document.getElementById("load")?.addEventListener("click", () => {
    if (loadGame(input.value)) {
      localStorage.setItem("chess-last-game", input.value);
    } else {
      alert("could not parse game JSON (paste the contents of game.json)");
    }
  });
  const slider = document.getElementById("ply") as HTMLInputElement;
  slider.addEventListener("input", () => {
    ply = Number(slider.value);
    showPly();
  });
  document.getElementById("first")?.addEventListener("click", () => {
    ply = 0;
    showPly();
  });
  document.getElementById("prev")?.addEventListener("click", () => {
    if (ply > 0) ply--;
    showPly();
  });
  document.getElementById("next")?.addEventListener("click", () => {
    if (ply < fens.length - 1) ply++;
    showPly();
  });
  document.getElementById("last")?.addEventListener("click", () => {
    ply = Math.max(fens.length - 1, 0);
    showPly();
  });
}

if (typeof document !== "undefined" && document.getElementById("board")) {
  setup();
}
