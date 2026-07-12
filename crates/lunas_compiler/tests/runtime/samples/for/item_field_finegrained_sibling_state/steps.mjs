export default async ({ $$, click, equal }) => {
  const L = () => $$("span.lbl").map((n) => n.innerHTMLString()).join(",");
  // Give each input a distinct value to detect any node re-creation.
  const inputs = $$("input.edit");
  inputs.forEach((el, i) => (el.value = "v" + i));
  // A deep field write on row 1 must patch ONLY that item's text bind.
  await click("button.bump");
  equal(L(), "a,b!,c");
  // Sibling input nodes must be the SAME nodes (state preserved -> field patch
  // never re-ran the reconciler / rebuilt items).
  const after = $$("input.edit");
  equal(after.map((el) => el.value).join(","), "v0,v1,v2");
  // A structural change (append) still reconciles correctly after field writes,
  // and preserves the existing items' input state.
  await click("button.add");
  equal(L(), "a,b!,c,z");
  const grown = $$("input.edit");
  equal(grown.map((el) => el.value).join(","), "v0,v1,v2,");
};
