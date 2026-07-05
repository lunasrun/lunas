export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  await click(".g");
  equal(L(), "1,2,3,4");
  await click(".s");
  equal(L(), "2,3");
};
