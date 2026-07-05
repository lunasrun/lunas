export default async ({ $$, click, equal }) => {
  const labels = () => $$("li").map((n) => n.innerHTMLString()).join(",");
  equal(labels(), "a,b");
  await click("button");
  equal(labels(), "a,b,c");
};
