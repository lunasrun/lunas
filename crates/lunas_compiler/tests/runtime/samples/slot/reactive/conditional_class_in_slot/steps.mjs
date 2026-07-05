export default async ({ $, click, expect }) => {
  expect("span").attr("class", "off");
  await click("button");
  expect("span").attr("class", "on");
};
