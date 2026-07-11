export default async ({ $$, equal }) => {
  const L = () => $$("option.opt").map(n => n.innerHTMLString()).join(",");
  equal(L(), "one,two");
};
