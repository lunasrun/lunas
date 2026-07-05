export default async ({ click, expect }) => {
  expect("button").text("c=1");
  expect("span").count(0);
  expect("li").count(2);
  await click("button");
  expect("button").text("c=2");
  expect("span").count(1);
  expect("span").text("big");
};
