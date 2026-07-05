export default async ({ $, click, expect }) => {
  expect($("span")).text("hi ada");
  await click("button");
  expect($("span")).text("hi grace");
};
