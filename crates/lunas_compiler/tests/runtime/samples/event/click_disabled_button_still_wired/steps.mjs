export default async ({ $, click, expect }) => {
  expect("button").prop("disabled", false);
  await click("button");
  expect("button").text("n=1");
};
