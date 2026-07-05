export default async ({ click, expect }) => {
  expect("article").html("<em>start</em>");
  await click("button");
  expect("article").html("<em>end</em>");
};
