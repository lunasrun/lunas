export default async ({ click, expect }) => {
  expect("p").text("Step One");
  await click("button");
  expect("p").text("Step Two");
  await click("button");
  expect("p").text("Step Three");
  await click("button");
  expect("p").text("Step One");
};
