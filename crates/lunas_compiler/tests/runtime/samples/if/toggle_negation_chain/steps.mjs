export default async ({ click, expect }) => {
  expect("p").text("Hidden");
  await click("button");
  expect("p").text("Visible");
  await click("button");
  expect("p").text("Hidden");
};
