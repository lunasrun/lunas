export default async ({ click, expect }) => {
  expect("p").text("before");
  await click("button");
  expect("p").attr("data-marker", "renamed");
};
