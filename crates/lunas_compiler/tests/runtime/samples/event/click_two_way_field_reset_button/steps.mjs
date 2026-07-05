export default async ({ $, click, setValue, expect }) => {
  expect("input").value("hello");
  await setValue("input", "world");
  expect("input").value("world");
  await click("button");
  expect("input").value("");
};
