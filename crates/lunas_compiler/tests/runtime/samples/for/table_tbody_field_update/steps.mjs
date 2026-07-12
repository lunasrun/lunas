export default async ({ $$, click, equal }) => {
  const L = () => $$("td.cell").map(n => n.innerHTMLString()).join(",");
  equal(L(), "a,b");
  await click($$("button.up")[0]);
  equal(L(), "a!,b");
  await click($$("button.up")[1]);
  equal(L(), "a!,b!");
};
