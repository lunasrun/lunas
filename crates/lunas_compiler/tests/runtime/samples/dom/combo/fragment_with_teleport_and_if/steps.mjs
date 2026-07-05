export default async ({ click, expect }) => {
  expect("h2").text("panel heading");
  const hasPorted = () => !!document.body.querySelector(".ported-fragment-teleport-if");
  if (hasPorted()) throw new Error("should start hidden");
  await click("button");
  if (!hasPorted()) throw new Error("should show after toggle");
};
