export default async ({ click, expect }) => {
  expect(".status").text("idle");
  await click(".save");
  expect(".status").text("saved");
  await click(".cancel");
  expect(".status").text("cancelled");
};
