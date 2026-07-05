export default async ({ $, click, expect }) => {
  // scoped prop is a build-time snapshot (50); the parent's n reacts.
  expect(".host").text("50/1");
  await click("button");
  expect(".host").text("50/2");
};
