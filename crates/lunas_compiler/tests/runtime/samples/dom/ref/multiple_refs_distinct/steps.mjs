export default async ({ $$, click, equal }) => {
  await click("button");
  const [first, second] = $$("p");
  equal(first.getAttribute("data-tag"), "first");
  equal(second.getAttribute("data-tag"), "second");
};
