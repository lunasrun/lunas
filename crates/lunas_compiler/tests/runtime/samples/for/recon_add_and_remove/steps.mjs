export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const [a,,c] = $$("li");
  await click(".go");
  equal(L(), "a,z,c");
  const now = $$("li");
  equal(now[0]===a && now[2]===c, true); // b removed, z inserted, a/c kept
};
