export default async ({ click, expect }) => {
  expect(".chart-view").text("chart:2");
  const status = () => document.body.querySelector(".ported-dashboard-status").innerHTMLString();
  expect("h1").attr("data-view", null);
  await click(".table");
  expect(".table-view").count(1);
  expect(".chart-view").count(0);
  expect("h1").attr("data-view", "table");
  if (status() !== "showing table") throw new Error("teleported status mismatch: " + status());
};
