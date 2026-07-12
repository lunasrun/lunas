export default async ({ $$, click, expect }) => {
  // Stress case combining all five DOM features at once: fragment (Shell has
  // several top-level nodes), :ref on the heading, :for list, dynamic
  // <component :is>, and a <teleport> with a reactive :html binding.
  expect("h1").text("Shell");
  expect(".row").count(2);
  expect(".foo").count(1);
  const ported = () => document.body.querySelector(".ported-stress");
  if (ported().innerHTMLString() !== "<b>stress init</b>") {
    throw new Error("initial teleported html mismatch: " + ported().innerHTMLString());
  }

  await click(".mark");
  expect("h1").attr("data-marked", "yes");

  await click(".swap");
  expect(".foo").count(0);
  expect(".bar").count(1);
  if (ported().innerHTMLString() !== "<i>stress swapped</i>") {
    throw new Error("post-swap teleported html mismatch: " + ported().innerHTMLString());
  }
  expect(".row").count(2);
};
