export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  await click(".rm");
  equal(L(), "a,c");
  await click(".ap");
  equal(L(), "a,c,d");
};
