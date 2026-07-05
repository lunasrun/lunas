export default async ({ click, expect }) => {
  expect("span").count(0);
  expect("p").count(0);
  await click("button");
  expect("span").count(1);
  expect("p").count(1);
  expect("span").text("Dark Icon");
  expect("p").text("Dark Label");
  await click("button");
  expect("span").count(0);
  expect("p").count(0);
};
