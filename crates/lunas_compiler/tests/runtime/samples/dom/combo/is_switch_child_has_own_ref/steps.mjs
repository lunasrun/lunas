export default async ({ click, expect }) => {
  // The child mounted by <component :is> owns its own :ref internally --
  // this is the supported form (:ref belongs to the CHILD's own template,
  // not the <component :is> call site).
  await click(".mark");
  expect(".foo").attr("data-marked", "yes");
  await click(".swap");
  expect(".bar").count(1);
  expect(".foo").count(0);
};
