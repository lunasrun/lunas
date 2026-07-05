import { component, bind, box, on as $on, refs, setClass, setStyle } from "lunas";

const HTML = "<div class=\"box\"></div>";

export default component("div", {}, HTML, (c, props) => {
  const on = box(c, 0, false)
  let huge = true
  const hue = box(c, 1, "red")
  let weight = "bold"
  function go() {
      on.v = !on.v
      hue.v = "blue"
  }
  const [e0] = refs(c.root, [[0]]);
  bind(c, [0], () => { setClass(e0, "box", { active: on.v, big: huge }); });
  bind(c, [1], () => { setStyle(e0, "", { color: hue.v, fontWeight: weight }); });
  $on(e0, "click", () => { go(); });
});
