export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  await click(".go");
  equal(L(), "2,1,1");
};
