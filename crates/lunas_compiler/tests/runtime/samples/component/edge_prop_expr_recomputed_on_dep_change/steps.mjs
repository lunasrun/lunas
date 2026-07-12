export default async ({ $, click, expect }) => {
  expect($("span")).text("3");
  await click(".a");
  expect($("span")).text("4");
  await click(".b");
  expect($("span")).text("5");
};
