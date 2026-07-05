import { fragment, anchorAppend, anchorBefore, bind, box, fromHTML, ifBlock, on, refs } from "lunas";

const HTML = "<h1></h1><footer>end</footer>";
const HTML_1 = "<p></p>";

export default fragment({}, HTML, (c, props) => {
  const title = box(c, 0, "T")
  const show = box(c, 1, true)
  function rename() {
      title.v = "T2"
      show.v = !show.v
  }
  const [e0] = refs(c.root, [[0]]);
  const [g0, g1] = refs(c.root, [[0], [1]]);
  const t0 = anchorAppend(g0);
  bind(c, [0], () => { t0.data = `${title.v}`; });
  const a0 = anchorBefore(g1);
  ifBlock(c, a0, [1], () => (show.v), () => {
    const r0 = fromHTML(HTML_1, a0);
    const [g2] = refs(r0, [[0]]);
    const t1 = anchorAppend(g2);
    bind(c, [0], () => { t1.data = `visible ${title.v}`; });
    return r0.childNodes[0];
  });
  on(e0, "click", () => { rename(); });
});
