export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const [a,b] = $$("li");
  await click(".go");
  equal(L(), "b,a");
  const now = $$("li");
  equal(now[0]===b && now[1]===a, true);
};
