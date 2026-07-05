export default async ({ $$, click, equal }) => {
  // Each :for item here is itself multi-node (a label <li> plus a
  // conditional detail <li>) -- fragments.md's ":for items with several
  // nodes" group machinery. Reordering the list must move each item's whole
  // node group together.
  const labels = () => $$(".label").map((n) => n.innerHTMLString()).join(",");
  equal(labels(), "a,b");
  equal($$(".detail").length, 1);
  equal($$(".detail")[0].innerHTMLString(), "detail-a");
  await click("button");
  equal(labels(), "b,a");
  equal($$(".detail").length, 1);
  equal($$(".detail")[0].innerHTMLString(), "detail-a");
};
