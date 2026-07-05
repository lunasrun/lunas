export default async ({ $, click, expect }) => {
  expect($("span")).text("lvl 1");
  await click("button");
  expect($("span")).text("lvl 2");
};
