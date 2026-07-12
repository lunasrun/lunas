import { component, anchorAppend, bind, fromHTML, prop, refs, slotBlock } from "lunas";

const HTML = "<section><header></header><main></main><footer></footer></section>";
const HTML_1 = "untitled";
const HTML_2 = "empty";

export default component("div", {}, HTML, (c, props) => {
  const tone = prop(c, "tone", 0, props.tone, ("plain"));
  let rows = 3
  const [e0] = refs(c.root, [[0]]);
  const [g0, g1, g2] = refs(c.root, [[0, 0], [0, 1], [0, 2]]);
  const a0 = anchorAppend(g0);
  slotBlock(c, a0, props.$slots && props.$slots["head"], (slotProps) => {
    const r0 = fromHTML(HTML_1, a0);
    return Array.from(r0.childNodes);
  });
  const a1 = anchorAppend(g1);
  slotBlock(c, a1, props.$slots && props.$slots["default"], (slotProps) => {
    const r1 = fromHTML(HTML_2, a1);
    return Array.from(r1.childNodes);
  });
  const a2 = anchorAppend(g2);
  slotBlock(c, a2, props.$slots && props.$slots["foot"], null, () => ({ count: (rows) }));
  bind(c, [0], () => { e0.setAttribute("class", `card ${tone.v}`); });
});
