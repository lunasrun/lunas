export default async ({ $, $$, click, expect }) => {
  expect($("p")).text("none");
  await click($$("button")[0]);
  // The child raised @save; the parent's onSave handler ran and updated state.
  expect($("p")).text("saved:42");
};
