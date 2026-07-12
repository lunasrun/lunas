export default async ({ $$, click, equal }) => {
const L = () => $$("span").map(n => n.innerHTMLString()).join(",");
  await click($$("button.del")[1]);
  equal(L(), "a,c");
  await click($$("button.del")[0]);
  equal(L(), "c");
};
