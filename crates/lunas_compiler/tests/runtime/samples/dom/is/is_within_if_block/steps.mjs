export default async ({ click, expect }) => {
  expect(".panel").count(1);
  await click(".toggle"); // hide
  expect(".panel").count(0);
  await click(".swap"); // change :is while hidden
  await click(".toggle"); // show again
  expect(".notice").count(1);
  expect(".panel").count(0);
};
