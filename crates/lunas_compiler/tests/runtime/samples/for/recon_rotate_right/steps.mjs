export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const [a,,c] = $$("li");
  await click(".go");
  equal(L(), "c,a,b");
  const now = $$("li");
  equal(now[0]===c && now[1]===a, true);
};
