export default async ({ click, expect }) => {
  expect("li").count(2);
  await click("button"); // hide
  expect("ul").count(0);
  expect("li").count(0);
  await click("button"); // reshow rebuilds the for-list fresh from `items`
  expect("li").count(2);
  expect("li").text("x");
};
