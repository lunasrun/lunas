export default async ({ $$, click, equal }) => {
  const state = () => $$("span").map(n => n.innerHTMLString()).join(",");
  equal(state(), "ON,off");
  await click(".go");
  equal(state(), "off,ON");
};
