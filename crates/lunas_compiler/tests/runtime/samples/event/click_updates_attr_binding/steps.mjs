export default async ({ click, expect }) => {
  expect(".box").attr("title", "idle");
  await click(".box");
  expect(".box").attr("title", "clicked");
};
