export default async ({ click, expect }) => {
  // <component :is> can mount a fragment (multi-root) child too -- swapping
  // unmounts/mounts the whole node group.
  expect(".a-head").count(1);
  expect(".a-body").count(1);
  await click("button");
  expect(".a-head").count(0);
  expect(".a-body").count(0);
  expect(".b-head").count(1);
  expect(".b-body").count(1);
};
