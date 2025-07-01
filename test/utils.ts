import Rand from "rand-seed";

export function pickWithSeed<T>(array: T[], count: number, seed: string): T[] {
  const rng = new Rand(seed);
  const res: T[] = [];
  for (let i = 0; i < count; i++) {
    const idx = Math.floor(rng.next() * array.length);
    res.push(array[idx]);
  }
  return res;
}
