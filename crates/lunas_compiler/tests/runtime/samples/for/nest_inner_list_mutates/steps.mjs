export default async ({ $$, click, equal }) => {
  const c = () => $$("span.c").map(n => n.innerHTMLString()).join(",");
  equal(c(), "1,2,3");
  await click(".go");
  equal(c(), "1,2,9,3");
};
