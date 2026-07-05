export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const [,b,c] = $$("li");
  await click(".go");
  equal(L(), "b,c");
  const now = $$("li");
  equal(now[0]===b && now[1]===c, true);
};
