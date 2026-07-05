export default async ({ $$, click, equal }) => {
const L = () => $$("span").map(n => n.innerHTMLString()).join(",");
  await click($$("button.inc")[0]);
  equal(L(), "1,0");
  await click($$("button.inc")[0]);
  equal(L(), "2,0");
};
