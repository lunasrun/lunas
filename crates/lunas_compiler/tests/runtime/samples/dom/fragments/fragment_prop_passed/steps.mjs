export default async ({ click, expect }) => {
  expect("h1").text("Main");
  expect("h2").text("Sub");
  await click("button");
  expect("h1").text("Renamed");
  expect("h2").text("Sub2");
};
