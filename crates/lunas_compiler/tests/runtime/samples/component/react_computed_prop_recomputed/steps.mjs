export default async ({ $, click, expect }) => {
  expect($("span")).text("2");
  await click("button");
  expect($("span")).text("4");
  await click("button");
  expect($("span")).text("6");
};
