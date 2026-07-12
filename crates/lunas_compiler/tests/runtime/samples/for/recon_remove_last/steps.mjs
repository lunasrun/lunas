export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const [a,b] = $$("li");
  await click(".go");
  equal(L(), "a,b");
  const now = $$("li");
  equal(now[0]===a && now[1]===b, true);
};
