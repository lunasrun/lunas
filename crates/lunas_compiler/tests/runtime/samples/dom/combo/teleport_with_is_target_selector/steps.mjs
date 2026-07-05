export default async ({ click, expect }) => {
  expect(".foo").count(1);
  const ported = document.body.querySelector(".ported-with-is-target");
  if (!ported) throw new Error("expected teleported content unaffected by :is");
  await click("button");
  expect(".foo").count(0);
  expect(".bar").count(1);
  if (!document.body.querySelector(".ported-with-is-target")) {
    throw new Error("teleported content should remain after unrelated :is swap");
  }
};
