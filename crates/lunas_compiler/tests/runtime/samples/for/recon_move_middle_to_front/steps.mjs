export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const [a,b,c] = $$("li");
  await click(".go");
  equal(L(), "b,a,c");
  const now = $$("li");
  equal(now[0]===b && now[1]===a && now[2]===c, true);
};
