import { component, anchorAppend, bind, box, dynamicBlock, fromHTML, on, refs, teleportBlock } from "lunas";
import Notice from "./Notice.lunas";
import Panel from "./Panel.lunas";

const HTML = "<div></div>";
const HTML_1 = "<p></p>";

export default component("div", {}, HTML, (c, props) => {
  const view = box(c, 0, Panel)
  let title = "hi"
  const msg = box(c, 1, "in portal")
  function swap() { view.v = Notice }
  function grow() { msg.v = "grew" }
  const [g0, g1] = refs(c.root, [[0], [0]]);
  const a0 = anchorAppend(g0);
  const ch0 = dynamicBlock(c, a0, [0], () => (view.v), { label: () => (title) });
  const a1 = anchorAppend(g1);
  teleportBlock(c, a1, () => (`#portal`), () => {
    const r0 = fromHTML(HTML_1, a1);
    const [e0] = refs(r0, [[0]]);
    const [g2] = refs(r0, [[0]]);
    const t0 = anchorAppend(g2);
    bind(c, [1], () => { t0.data = `${msg.v}`; });
    on(e0, "click", () => { grow(); });
    return Array.from(r0.childNodes);
  });
});
