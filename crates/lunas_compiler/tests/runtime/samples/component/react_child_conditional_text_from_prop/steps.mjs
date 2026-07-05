export default async ({ $, click, expect }) => {
  expect($("span")).text("empty");
  await click("button");
  expect($("span")).text("1");
};
