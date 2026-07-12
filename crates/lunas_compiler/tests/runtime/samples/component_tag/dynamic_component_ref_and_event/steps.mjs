export default async ({ $, $$, click, expect }) => {
  expect($("p")).text("none");
  await click($$("button")[0]);
  // @save on <component :is> is wired: the handler ran (:ref did NOT swallow it).
  expect($("p")).text("got:7");
};
