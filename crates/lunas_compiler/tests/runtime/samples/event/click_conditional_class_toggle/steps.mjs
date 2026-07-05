export default async ({ $, click, expect }) => {
  expect("button").attr("class", "btn");
  await click("button");
  expect("button").attr("class", "btn on");
};
