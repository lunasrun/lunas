export default async ({ $$, click, equal }) => {
  await click("button");
  // Discover behavior: whichever :for item's ref won the shared variable
  // (per-item ref assignment order), exactly one <li> ends up marked.
  const marked = $$("li").filter((n) => n.getAttribute("data-marked") === "yes");
  equal(marked.length, 1);
};
