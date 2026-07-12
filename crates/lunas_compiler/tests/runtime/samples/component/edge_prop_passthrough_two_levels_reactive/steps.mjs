export default async ({ $, click, expect }) => {
  expect($("b")).text("one");
  await click("button");
  expect($("b")).text("two");
};
