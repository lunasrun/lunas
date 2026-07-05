export default async ({ $, $$, setValue, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  equal(L(), "1:x,2:x");
  await setValue(".ctl", "z");
  equal(L(), "1:z,2:z");
};
