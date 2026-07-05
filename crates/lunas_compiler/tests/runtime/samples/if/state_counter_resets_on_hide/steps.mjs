// `n` is component-level reactive state (not scoped to the branch), so it does
// NOT reset when the branch is torn down and rebuilt — only the DOM nodes are
// recreated. This documents that the *value* survives across hide/reshow while
// the DOM subtree itself is a fresh mount each time.
export default async ({ $$, click, expect }) => {
  const toggleBtn = () => $$("button")[0];
  const bumpBtn = () => $$("button")[1];
  expect("span").text("0");
  await click(bumpBtn());
  await click(bumpBtn());
  expect("span").text("2");
  await click(toggleBtn()); // hide
  expect("span").count(0);
  await click(toggleBtn()); // reshow
  expect("span").text("2");
};
