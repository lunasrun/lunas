export default async ({ $, click, setValue, expect }) => {
  expect("input").value("default");
  await setValue("input", "typed");
  expect("input").value("typed");
  await click("button");
  expect("input").value("default");
};
