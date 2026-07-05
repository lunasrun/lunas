import { component, anchorAppend, bind, box, fromHTML, ifChain, on, refs } from "lunas";

const HTML = "<div><button>step</button></div>";
const HTML_1 = "<p>zero</p>";
const HTML_2 = "<p></p>";
const HTML_3 = "<p></p>";

export default component("div", {}, HTML, (c, props) => {
  const n = box(c, 0, 0)
  function bump() { n.v = n.v + 1 }
  const [e0] = refs(c.root, [[0, 0]]);
  const [g0] = refs(c.root, [[0]]);
  const a0 = anchorAppend(g0);
  ifChain(c, a0, [0], () => (n.v == 0) ? 0 : (n.v > 3) ? 1 : 2, [
    () => {
      const r0 = fromHTML(HTML_1, a0);
      return r0.childNodes[0];
    },
    () => {
      const r1 = fromHTML(HTML_2, a0);
      const [g1] = refs(r1, [[0]]);
      const t0 = anchorAppend(g1);
      bind(c, [0], () => { t0.data = `big ${n.v}`; });
      return r1.childNodes[0];
    },
    () => {
      const r2 = fromHTML(HTML_3, a0);
      const [g2] = refs(r2, [[0]]);
      const t1 = anchorAppend(g2);
      bind(c, [0], () => { t1.data = `small ${n.v}`; });
      return r2.childNodes[0];
    },
  ]);
  on(e0, "click", () => { bump(); });
});
