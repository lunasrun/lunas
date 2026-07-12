export default async ({ click }) => {
  const el = () => document.body.querySelector(".ported-class-binding");
  const hasActive = () =>
    (el().getAttribute("class") || "").split(/\s+/).includes("active");
  if (hasActive()) throw new Error("should not start active");
  await click("button");
  if (!hasActive()) throw new Error("should become active after toggle");
};
