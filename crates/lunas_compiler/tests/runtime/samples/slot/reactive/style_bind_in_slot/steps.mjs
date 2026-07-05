export default async ({ $, click, expect }) => {
  expect("span").attr("style", "width:10px");
  await click("button");
  expect("span").attr("style", "width:15px");
};
