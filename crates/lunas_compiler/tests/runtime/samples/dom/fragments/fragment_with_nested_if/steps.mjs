export default async ({ click, expect }) => {
  expect("p").text("deep detail");
  await click("button");
  expect("p").count(0);
};
