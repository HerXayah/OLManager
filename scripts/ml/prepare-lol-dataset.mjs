import fs from "node:fs";
import path from "node:path";
import readline from "node:readline";
import os from "node:os";
import crypto from "node:crypto";

const inputDir = process.argv[2] || path.join(os.tmpdir(), "olmanager", "lol-sim-telemetry");
const outputDir = process.argv[3] || path.join(os.tmpdir(), "olmanager", "nn-datasets");

const SPLITS = [
  { name: "train", max: 0.8 },
  { name: "val", max: 0.9 },
  { name: "test", max: 1.0 },
];

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function pickSplit(seed, sessionId) {
  const base = `${seed ?? ""}|${sessionId ?? ""}`;
  const hashHex = crypto.createHash("sha1").update(base).digest("hex").slice(0, 8);
  const n = parseInt(hashHex, 16) / 0xffffffff;
  return SPLITS.find((s) => n < s.max)?.name || "train";
}

function createWriterGroup(prefix) {
  return {
    train: fs.createWriteStream(path.join(outputDir, `${prefix}-train.jsonl`), { encoding: "utf8" }),
    val: fs.createWriteStream(path.join(outputDir, `${prefix}-val.jsonl`), { encoding: "utf8" }),
    test: fs.createWriteStream(path.join(outputDir, `${prefix}-test.jsonl`), { encoding: "utf8" }),
  };
}

function closeWriterGroup(group) {
  for (const stream of Object.values(group)) {
    stream.end();
  }
}

async function main() {
  if (!fs.existsSync(inputDir)) {
    throw new Error(`Input directory not found: ${inputDir}`);
  }

  ensureDir(outputDir);

  const files = fs
    .readdirSync(inputDir)
    .filter((name) => name.toLowerCase().endsWith(".jsonl"))
    .map((name) => path.join(inputDir, name));

  if (!files.length) {
    throw new Error(`No .jsonl files found in: ${inputDir}`);
  }

  const allWriters = createWriterGroup("lol-nn-all");
  const hybridWriters = createWriterGroup("lol-nn-hybrid");

  const stats = {
    inputDir,
    outputDir,
    files: files.length,
    totalLines: 0,
    parsedLines: 0,
    invalidLines: 0,
    all: { train: 0, val: 0, test: 0 },
    hybrid: { train: 0, val: 0, test: 0 },
  };

  for (const filePath of files) {
    const rl = readline.createInterface({
      input: fs.createReadStream(filePath, { encoding: "utf8" }),
      crlfDelay: Infinity,
    });

    for await (const line of rl) {
      stats.totalLines += 1;
      if (!line || !line.trim()) continue;

      let row;
      try {
        row = JSON.parse(line);
      } catch {
        stats.invalidLines += 1;
        continue;
      }

      stats.parsedLines += 1;
      const split = pickSplit(row.seed, row.sessionId);

      allWriters[split].write(JSON.stringify(row) + "\n");
      stats.all[split] += 1;

      if (row.aiMode === "hybrid") {
        hybridWriters[split].write(JSON.stringify(row) + "\n");
        stats.hybrid[split] += 1;
      }
    }
  }

  closeWriterGroup(allWriters);
  closeWriterGroup(hybridWriters);

  const manifestPath = path.join(outputDir, "lol-nn-dataset-manifest.json");
  fs.writeFileSync(manifestPath, JSON.stringify(stats, null, 2), "utf8");

  console.log("Dataset prepared successfully");
  console.log(JSON.stringify(stats, null, 2));
}

main().catch((error) => {
  console.error("Dataset preparation failed:", error?.message || error);
  process.exit(1);
});
