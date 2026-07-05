export default async ({ $, click, expect }) => {
  expect($("b")).text("1");
  await click("button");
  expect($("b")).text("2");
};
