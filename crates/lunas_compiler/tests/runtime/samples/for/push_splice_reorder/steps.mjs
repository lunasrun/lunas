export default async ({ $$, click, tick, equal }) => {
  const labels = () => $$("li").map((n) => n.innerHTMLString()).join(",");
  const [p, d, r] = $$("button");
  equal(labels(), "a,b");
  await click(p);
  equal(labels(), "a,b,c");
  await click(d);
  equal(labels(), "a,c");
  await click(r);
  equal(labels(), "c,a");
};
