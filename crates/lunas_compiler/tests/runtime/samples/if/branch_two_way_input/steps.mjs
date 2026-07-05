export default async ({ $, click, setValue, expect }) => {
  expect("input").count(0);
  await click("button");
  expect("input").count(1);
  expect("span").text("hi");
  await setValue("input", "bye");
  expect("span").text("bye");
};
