import { beforeAll, beforeEach, expect } from "vitest";
import { Biome, Distribution } from "@biomejs/js-api";
import { diffLines } from "diff";
import pc from "picocolors";

let biome: Biome;

// テスト開始前に Biome インスタンスを初期化
beforeAll(async () => {
  biome = await Biome.create({
    distribution: Distribution.NODE,
  });
});

beforeEach(() => {
  document.body.innerHTML = "";
});

/** Biome で HTML をフォーマットし、trim して返す */
function formatHtmlWithBiome(html: string): string {
  return biome.formatContent(1, html, { filePath: "file.html" }).content.trim();
}

/** diffLines を使ってカラフルな差分を作成 */
function formatDiff(expected: string, received: string): string {
  return diffLines(expected, received)
    .map((part) => {
      if (part.added) return pc.green("+ " + part.value);
      if (part.removed) return pc.red("- " + part.value);
      return "  " + part.value;
    })
    .join("");
}

// カスタムマッチャーを同期関数で定義
expect.extend({
  toEqualNormalizedHtml(received: string, expected: string) {
    const expFmt = formatHtmlWithBiome(expected);
    const recFmt = formatHtmlWithBiome(received);

    const pass = expFmt === recFmt;
    const diff = formatDiff(expFmt, recFmt);

    return {
      pass,
      message: () =>
        pass
          ? pc.green("✅ HTML matched")
          : pc.red("❌ HTML mismatch:\n\n") + diff,
    };
  },
});
