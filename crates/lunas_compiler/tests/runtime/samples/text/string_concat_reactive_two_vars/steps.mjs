export default async ({ click, expect }) => {
  expect("p").text("v1 static");
  await click("button");
  expect("p").text("v2 static");
};
