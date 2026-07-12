export default async ({ $$, click, equal }) => {
  const labels = () => $$("li").map((n) => n.innerHTMLString()).join(",");
  equal(labels(), "a,b,c");
  await click("button");
  equal(labels(), "c,b,a");
};
