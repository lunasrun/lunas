export default async ({ $, $$, click, expect, equal }) => {
  // Branch shown (k=1>0): child renders with reactive prop n.
  expect($("span")).text("n=1");
  // Bump k: still shown, and the reactive prop updates in place (no remount).
  await click("button");
  expect($("span")).text("n=2");
  equal($$("span").length, 1, "same child instance, prop updated");
};
