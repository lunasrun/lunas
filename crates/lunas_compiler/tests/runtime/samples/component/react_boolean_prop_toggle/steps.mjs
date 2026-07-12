export default async ({ $, click, expect }) => {
  expect($("span")).text("OFF");
  await click("button");
  expect($("span")).text("ON");
  await click("button");
  expect($("span")).text("OFF");
};
