import { component, anchorAppend, anchorBefore, bind, deepBox, forBlock, fromHTML, ifBlock, on, refs } from "lunas";

const HTML = "<div><button>add</button><button>toggle</button><ul></ul></div>";
const HTML_1 = "<li><b></b><ol></ol></li>";
const HTML_2 = "<em>(open)</em>";
const HTML_3 = "<li></li>";

export default component("div", {}, HTML, (c, props) => {
  const groups = deepBox(c, 0, [
      { id: 1, name: "a", open: true, tags: ["x", "y"] },
      { id: 2, name: "b", open: false, tags: ["z"] }
  ])
  function add() {
      (groups.touch(), groups.v.push({ id: groups.v.length + 1, name: "n", open: false, tags: ["t"] }))
  }
  function toggleFirst() { (groups.touchElem(groups.v[0]), groups.v[0].open = !groups.v[0].open) }
  const [e0, e1] = refs(c.root, [[0, 0], [0, 1]]);
  const [g0] = refs(c.root, [[0, 2]]);
  const a0 = anchorAppend(g0);
  forBlock(c, a0, [0], () => Array.from((groups.v) || []), {
    html: HTML_1,
    box: groups,
    wire: (r0, d0) => {
      let group = d0;
      const [g1, g2, g3] = refs(r0, [[0], [1], [1]]);
      const t0 = anchorAppend(g1);
      bind(c, [], () => { t0.data = `${group.name}`; });
      const a1 = anchorBefore(g2);
      ifBlock(c, a1, [], () => (group.open), () => {
        const r1 = fromHTML(HTML_2, a1);
        return r1.childNodes[0];
      });
      const a2 = anchorAppend(g3);
      forBlock(c, a2, [], () => Array.from((group.tags) || []), {
        html: HTML_3,
        wire: (r2, d2) => {
          let tag = d2;
          const [g4] = refs(r2, [[]]);
          const t1 = anchorAppend(g4);
          bind(c, [], () => { t1.data = `${tag}`; });
          return (d3) => { (tag = d3); };
        },
        keyOf: (d4) => { const tag = d4; return (tag); },
      });
      return (d1) => { (group = d1); };
    },
    keyOf: (d5) => { const group = d5; return (group.id); },
  });
  on(e0, "click", () => { add(); });
  on(e1, "click", () => { toggleFirst(); });
});
