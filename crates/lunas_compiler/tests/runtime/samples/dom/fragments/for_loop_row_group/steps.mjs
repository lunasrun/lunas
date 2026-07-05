export default async ({ $$, click, equal }) => {
  // Note (gap): a component tag (`<Row :for="...">`) is NOT yet supported --
  // the compiler warns "`:for` on a component is not supported yet" -- so a
  // fragment component cannot currently be mounted per-:for-item. This case
  // instead covers the supported adjacent form: multiple plain elements that
  // each carry their own `:for` binding over the same list, which is the
  // node-group machinery fragments.md describes for :for items.
  const labels = () => $$(".main").map((tr) => tr.innerHTMLString());
  equal(labels().join(","), "<td>a</td>,<td>b</td>");
  await click("button");
  equal(labels().join(","), "<td>a</td>,<td>b</td>,<td>c</td>");
};
