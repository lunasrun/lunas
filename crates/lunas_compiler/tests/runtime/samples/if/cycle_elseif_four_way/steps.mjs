export default async ({ click, expect }) => {
  expect("p").text("Mode A");
  await click("button");
  expect("p").text("Mode B");
  await click("button");
  expect("p").text("Mode C");
  await click("button");
  expect("p").text("Mode D");
  await click("button");
  expect("p").text("Mode A");
};
