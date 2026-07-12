export default async ({ $, click, expect }) => {
  expect($("button")).text("0");
  await click("button");
  expect($("button")).text("3");
  await click("button");
  expect($("button")).text("6");
};
