export default async ({ $, click, expect }) => {
  expect(".ba").text("0");
  expect(".bb").text("100");
  await click(".ba");
  expect(".ba").text("1");
  expect(".bb").text("100");
  await click(".bb");
  expect(".ba").text("1");
  expect(".bb").text("99");
};
