import { component, anchorAppend, bind, prop, refs } from "lunas";

const HTML = "<span class=\"badge\"></span>";

export default component("div", {}, HTML, (c, props) => {
  const text = prop(c, "text", 0, props.text, ("?"));
  const [g0] = refs(c.root, [[0]]);
  const t0 = anchorAppend(g0);
  bind(c, [0], () => { t0.data = `[${text.v}]`; });
});
