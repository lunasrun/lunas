export default async ({ click, expect }) => {
  // Child emit("changed", 1) runs the parent's onChanged, which mutates parent
  // state — that write is what re-renders the parent's total text.
  expect("p").text("total: 0");
  await click("button");
  expect("p").text("total: 1");
  await click("button");
  await click("button");
  expect("p").text("total: 3");
};
