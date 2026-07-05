import { component, anchorAppend, bind, box, on, prop, refs } from "lunas";

const HTML = "<div class=\"counter\"><button>-</button><span class=\"value\"></span><button>+</button><input><p class=\"note\"></p></div>";

export default component("div", {}, HTML, (c, props) => {
  const start = prop(c, "start", 2, props.start, (0));
  const value = box(c, 0, start.v)
  const note = box(c, 1, "x")
  function inc() { value.v = value.v + 1 }
  function dec() { value.v = value.v - 1 }
  const [e0, e1, e2] = refs(c.root, [[0, 0], [0, 2], [0, 3]]);
  const [g0, g1] = refs(c.root, [[0, 1], [0, 4]]);
  const t0 = anchorAppend(g0);
  bind(c, [0], () => { t0.data = `${value.v}`; });
  const t1 = anchorAppend(g1);
  bind(c, [1], () => { t1.data = `note: ${note.v}`; });
  on(e0, "click", () => { dec(); });
  on(e1, "click", () => { inc(); });
  bind(c, [1], () => { e2.value = note.v; });
  on(e2, "input", () => { note.v = e2.value; });
});
