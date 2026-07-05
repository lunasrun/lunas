import { component, anchorAppend, bind, box, on, refs } from "lunas";

const HTML = "<div><h1></h1><p></p><button>bump</button></div>";

export default component("div", {}, HTML, (c, props) => {
  let greeting = "hello"
  let name = "world"
  let label = "the heading"
  let flavor = "sweet"
  const count = box(c, 0, 0)
  function inc() { count.v = count.v + 1 }
  const [e0, e1, e2] = refs(c.root, [[0, 0], [0, 1], [0, 2]]);
  const [g0, g1] = refs(c.root, [[0, 0], [0, 1]]);
  const t0 = anchorAppend(g0);
  t0.data = `${greeting}, ${name}!`;
  const t1 = anchorAppend(g1);
  bind(c, [0], () => { t1.data = `count is ${count.v}`; });
  e0.setAttribute("title", label);
  e1.setAttribute("class", `tag ${flavor} end`);
  on(e2, "click", () => { inc(); });
});
