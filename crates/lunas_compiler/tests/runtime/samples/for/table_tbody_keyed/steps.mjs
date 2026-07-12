export default async ({ $$, click, equal }) => {
  const L = () => $$("td.cell").map(n => n.innerHTMLString()).join(",");
  // initial render already asserted via expected.html
  equal(L(), "a,b,c");
  // structural change: remove the middle row via its own button
  await click($$("button.del")[1]);
  equal(L(), "a,c");
  await click($$("button.del")[0]);
  equal(L(), "c");
};
