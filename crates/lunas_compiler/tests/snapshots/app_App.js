import { component, anchorAppend, anchorBefore, bind, box, deepBox, dynamicBlock, forBlock, fromHTML, ifChain, mountChild, on, refs, slotContent } from "lunas";
import Badge from "./Badge.lunas";
import Card from "./Card.lunas";
import Counter from "./Counter.lunas";

const HTML = "<main class=\"app\"><h1></h1><nav><button>one</button><button>two</button><button>add tag</button></nav><ul class=\"tags\"></ul></main>";
const HTML_1 = "";
const HTML_2 = "";
const HTML_3 = "";
const HTML_4 = "<p>Page one is active.</p>";
const HTML_5 = "<p>Page two is active.</p>";
const HTML_6 = "<p>No page selected.</p>";
const HTML_7 = "<li></li>";

export default component("div", {}, HTML, (c, props) => {
  let title = "Lunas E2E App"
  let user = "ada"
  let seed = 10
  const page = box(c, 0, "one")
  const tags = deepBox(c, 1, [
      { id: 1, label: "alpha" },
      { id: 2, label: "beta" }
  ])
  function show(p) { page.v = p }
  function addTag() {
      tags.v.push({ id: tags.v.length + 1, label: "extra" })
  }
  const [e0, e1, e2] = refs(c.root, [[0, 1, 0], [0, 1, 1], [0, 1, 2]]);
  const [g0, g1, g2, g3, g4] = refs(c.root, [[0, 0], [0, 2], [0, 2], [0, 2], [0]]);
  const t0 = anchorAppend(g0);
  t0.data = `${title}`;
  const a0 = anchorBefore(g1);
  const s0 = {
    default: (slotProps, onCleanup) => slotContent(c, (slotProps) => {
      const r0 = fromHTML(HTML_1, c.root);
      const [g5, g6] = refs(r0, [[], []]);
      const t1 = anchorAppend(g5);
      t1.data = `
            The counter starts at ${seed}.
            `;
      const a1 = anchorAppend(g6);
      const ch0 = mountChild(c, a1, Counter, { start: () => (seed) });
      return Array.from(r0.childNodes);
    }, slotProps, onCleanup),
    head: (slotProps, onCleanup) => slotContent(c, (slotProps) => {
      const r1 = fromHTML(HTML_2, c.root);
      const [g7] = refs(r1, [[]]);
      const t2 = anchorAppend(g7);
      t2.data = `Dashboard for ${user}`;
      return Array.from(r1.childNodes);
    }, slotProps, onCleanup),
    foot: (slotProps, onCleanup) => slotContent(c, (s) => {
      const r2 = fromHTML(HTML_3, c.root);
      const [g8] = refs(r2, [[]]);
      const t3 = anchorAppend(g8);
      t3.data = `rows: ${s.count}`;
      return Array.from(r2.childNodes);
    }, slotProps, onCleanup),
  };
  const ch1 = mountChild(c, a0, Card, { tone: `lead`, $slots: s0 });
  const a2 = anchorBefore(g2);
  ifChain(c, a2, [0], () => (page.v == 'one') ? 0 : (page.v == 'two') ? 1 : 2, [
    () => {
      const r3 = fromHTML(HTML_4, a2);
      return r3.childNodes[0];
    },
    () => {
      const r4 = fromHTML(HTML_5, a2);
      return r4.childNodes[0];
    },
    () => {
      const r5 = fromHTML(HTML_6, a2);
      return r5.childNodes[0];
    },
  ]);
  const a3 = anchorAppend(g3);
  forBlock(c, a3, [1], () => Array.from((tags.v) || []), {
    html: HTML_7,
    box: tags,
    wire: (r6, d0) => {
      let tag = d0;
      const [g9] = refs(r6, [[]]);
      const a4 = anchorAppend(g9);
      const ch2 = mountChild(c, a4, Badge, { text: () => (tag.label) });
      bind(c, [], () => { ch2.setProp("text", tag.label); });
      return (d1) => { (tag = d1); };
    },
    keyOf: (d2) => { const tag = d2; return (tag.id); },
  });
  const a5 = anchorAppend(g4);
  const ch3 = dynamicBlock(c, a5, [], () => (Badge), { text: () => (user) });
  on(e0, "click", () => { show('one'); });
  on(e1, "click", () => { show('two'); });
  on(e2, "click", () => { addTag(); });
});
