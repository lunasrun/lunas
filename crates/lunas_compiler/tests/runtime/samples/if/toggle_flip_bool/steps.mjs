export default async ({ $$, click, expect }) => {
  expect("p").count(1);
  expect("p").text("OFF");
  await click("button");
  expect("p").count(1);
  expect("p").text("ON");
  await click("button");
  expect("p").text("OFF");
};
