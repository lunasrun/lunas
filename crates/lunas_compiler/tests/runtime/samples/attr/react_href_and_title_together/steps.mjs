export default async ({ expect, click }) => {
  expect("a").attr("href", "/step1").attr("title", "Step 1");
  await click("a");
  expect("a").attr("href", "/step2").attr("title", "Step 2");
};
