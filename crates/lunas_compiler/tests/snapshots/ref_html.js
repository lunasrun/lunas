import { component, bind, box, deepBox, on, refs } from "lunas";

const HTML = "<div><input><article></article><button>fill</button></div>";

export default component("div", {}, HTML, (c, props) => {
  const field = deepBox(c, 0, undefined)
  const markup = box(c, 1, "<b>bold</b>")
  function fill() {
      field.v.value = "typed"
      markup.v = "<i>italic</i>"
  }
  const [e0, e1, e2] = refs(c.root, [[0, 0], [0, 1], [0, 2]]);
  field.v = e0;
  bind(c, [1], () => { e1.innerHTML = markup.v; });
  on(e2, "click", () => { fill(); });
});
