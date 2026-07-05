export default async ({ $, click, expect }) => {
  expect($("button")).text("");
  await click("button");
  expect($("button")).text("hi");
};
