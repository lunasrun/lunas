export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const three = $$("li")[2];
  await click(".go");
  equal(L(), "3");
  equal($$("li")[0] === three, true); // survivor keeps identity
};
