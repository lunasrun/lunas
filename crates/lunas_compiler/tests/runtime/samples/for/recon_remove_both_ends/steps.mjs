export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const [,b] = $$("li");
  await click(".go");
  equal(L(), "b");
  equal($$("li")[0]===b, true);
};
