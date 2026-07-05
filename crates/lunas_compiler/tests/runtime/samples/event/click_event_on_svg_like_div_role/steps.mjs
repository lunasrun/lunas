export default async ({ click, expect }) => {
  expect(".clickable").text("0");
  await click(".clickable");
  expect(".clickable").text("1");
};
