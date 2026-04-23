import fs from "node:fs";
import path from "node:path";
import readline from "node:readline";
import os from "node:os";

const datasetDir = process.argv[2] || path.join(os.tmpdir(), "olmanager", "nn-datasets");
const inputFile = process.argv[3] || path.join(datasetDir, "lol-nn-hybrid-train.jsonl");
const outputFile = process.argv[4] || path.join(datasetDir, "lol-policy-tuned.json");

function clamp01(v) {
  return Math.max(0, Math.min(1, v));
}

function isGoodOpenOutcome(outcome) {
  const death = Number(outcome?.deathDelta ?? 0);
  const hpDelta = Number(outcome?.hpRatioDelta ?? 0);
  const enemyHpDelta = Number(outcome?.enemyHpRatioDelta ?? 0);
  const kp = Number(outcome?.killParticipationDelta ?? 0);
  return death === 0 && hpDelta > -0.55 && (enemyHpDelta < -0.12 || kp > 0);
}

function shouldDisengage(outcome) {
  const death = Number(outcome?.deathDelta ?? 0);
  const hpDelta = Number(outcome?.hpRatioDelta ?? 0);
  return death > 0 || hpDelta < -0.28;
}

function scoreOpen(sample, threshold) {
  const predictOpen = sample.confidence >= threshold;
  const good = sample.label;
  if (predictOpen && good) return 2.0;
  if (predictOpen && !good) return -1.2;
  if (!predictOpen && good) return -0.9;
  return 0.4;
}

function scoreDisengage(sample, threshold) {
  const predictDisengage = sample.confidence <= threshold;
  const should = sample.label;
  if (predictDisengage && should) return 2.0;
  if (predictDisengage && !should) return -0.9;
  if (!predictDisengage && should) return -1.4;
  return 0.35;
}

async function loadSamples(filePath) {
  if (!fs.existsSync(filePath)) {
    throw new Error(`Dataset file not found: ${filePath}`);
  }

  const openSamples = [];
  const disengageSamples = [];
  const rl = readline.createInterface({
    input: fs.createReadStream(filePath, { encoding: "utf8" }),
    crlfDelay: Infinity,
  });

  for await (const line of rl) {
    if (!line.trim()) continue;
    let row;
    try {
      row = JSON.parse(line);
    } catch {
      continue;
    }

    if (row?.aiMode !== "hybrid" || row?.ruleDecision !== false) continue;

    const confidence = clamp01(Number(row?.confidence ?? 0));
    const intent = String(row?.intent ?? "");
    const outcome = row?.outcome ?? {};

    if (intent === "open-trade") {
      openSamples.push({ confidence, label: isGoodOpenOutcome(outcome) });
    } else if (intent === "disengage") {
      disengageSamples.push({ confidence, label: shouldDisengage(outcome) });
    }
  }

  return { openSamples, disengageSamples };
}

function gridSearch(samples, kind) {
  let bestThreshold = kind === "open" ? 0.64 : 0.38;
  let bestScore = Number.NEGATIVE_INFINITY;

  const min = kind === "open" ? 0.35 : 0.2;
  const max = kind === "open" ? 0.8 : 0.55;

  for (let t = min; t <= max; t += 0.01) {
    let score = 0;
    for (const sample of samples) {
      score += kind === "open" ? scoreOpen(sample, t) : scoreDisengage(sample, t);
    }
    if (score > bestScore) {
      bestScore = score;
      bestThreshold = Number(t.toFixed(2));
    }
  }

  return { bestThreshold, bestScore: Number(bestScore.toFixed(2)) };
}

async function main() {
  const { openSamples, disengageSamples } = await loadSamples(inputFile);
  if (!openSamples.length || !disengageSamples.length) {
    throw new Error("Not enough hybrid borderline samples to tune thresholds.");
  }

  const open = gridSearch(openSamples, "open");
  const disengage = gridSearch(disengageSamples, "disengage");

  const result = {
    source: inputFile,
    samples: {
      open: openSamples.length,
      disengage: disengageSamples.length,
    },
    tuned: {
      hybridOpenTradeConfidenceHigh: open.bestThreshold,
      hybridDisengageConfidenceLow: disengage.bestThreshold,
    },
    scores: {
      openObjective: open.bestScore,
      disengageObjective: disengage.bestScore,
    },
    generatedAt: new Date().toISOString(),
  };

  fs.writeFileSync(outputFile, JSON.stringify(result, null, 2), "utf8");
  console.log(JSON.stringify(result, null, 2));
}

main().catch((error) => {
  console.error(error?.message || error);
  process.exit(1);
});
