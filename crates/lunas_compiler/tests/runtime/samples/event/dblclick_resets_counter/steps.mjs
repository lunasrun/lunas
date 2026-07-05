export default async ({ $, click, dispatch, expect }) => {
  await click("button");
  await click("button");
  expect("button").text("n=2");
  await dispatch("button", "dblclick");
  expect("button").text("n=0");
};
