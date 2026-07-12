// Un-bound local DOM state (an uncommitted input value with no ::value bind)
// is genuinely destroyed on hide: the element itself is torn down, so a fresh
// <input> mounts empty on reshow.
export default async ({ $, click, setValue, expect }) => {
  expect("input").count(1);
  await setValue("input", "scratch note");
  expect("input").value("scratch note");
  await click("button"); // hide
  expect("input").count(0);
  await click("button"); // reshow
  expect("input").count(1);
  expect("input").value("");
};
