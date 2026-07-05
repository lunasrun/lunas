import { component, anchorAppend, bind, box, on, refs } from "lunas";

const HTML = "<form><input><input type=\"checkbox\"><p></p></form>";

export default component("div", {}, HTML, (c, props) => {
  const name = box(c, 0, "ada")
  const agree = box(c, 1, false)
  const [e0, e1] = refs(c.root, [[0, 0], [0, 1]]);
  const [g0] = refs(c.root, [[0, 2]]);
  const t0 = anchorAppend(g0);
  bind(c, [0, 1], () => { t0.data = `name=${name.v} agree=${agree.v}`; });
  bind(c, [0], () => { e0.value = name.v; });
  on(e0, "input", () => { name.v = e0.value; });
  bind(c, [1], () => { e1.checked = !!(agree.v); });
  on(e1, "change", () => { agree.v = e1.checked; });
});
