export default async ({ $$, click, equal }) => {
  const V = () => $$("span.v").map(n => n.innerHTMLString()).join(",");
  await click($$("button.inc")[1]);
  await click($$("button.inc")[1]);
  await click($$("button.inc")[2]);
  equal(V(), "0,2,1");
};
