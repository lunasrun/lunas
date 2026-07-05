export default async ({ click, expect }) => {
  expect("span").count(0);
  await click("button");
  expect("span").count(1);
  expect("span").attr("class", "tag");
  await click("button");
  expect("span").count(0);
  await click("button");
  expect("span").count(1);
  expect("span").attr("class", "tag");
};
